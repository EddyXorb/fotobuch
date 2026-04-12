//! Persistent Typst world with mtime-based slot invalidation and comemo caching.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use anyhow::Result;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{FontSlot, Fonts};

const EVICT_MAX_AGE: usize = 10;

struct FileSlot {
    mtime: SystemTime,
    len: u64,
    source: OnceCell<Source>,
    bytes: OnceCell<Bytes>,
    last_accessed: u32,
}

impl FileSlot {
    fn new(mtime: SystemTime, len: u64) -> Self {
        Self {
            mtime,
            len,
            source: OnceCell::new(),
            bytes: OnceCell::new(),
            last_accessed: 0,
        }
    }

    fn is_stale(&self, mtime: SystemTime, len: u64) -> bool {
        self.mtime != mtime || self.len != len
    }

    fn invalidate(&mut self, mtime: SystemTime, len: u64) {
        self.mtime = mtime;
        self.len = len;
        self.source = OnceCell::new();
        self.bytes = OnceCell::new();
    }
}

/// A Typst [`World`] implementation that keeps font metadata and file contents
/// across multiple compile calls. Only files whose `(mtime, len)` pair has
/// changed are re-read from disk; unchanged files reuse their cached
/// [`Source`] / [`Bytes`] values, keeping the comemo hash stable.
pub struct TypstWorld {
    root: PathBuf,
    main_id: FileId,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
    slots: Mutex<HashMap<FileId, FileSlot>>,
    revision: AtomicU32,
    /// Number of disk reads (test instrumentation).
    #[cfg(test)]
    pub reads: AtomicU32,
}

impl TypstWorld {
    /// Creates a world rooted at `project_root` whose main file is
    /// `{project_root}/{project_name}.typ`.
    pub fn new(project_root: &Path, project_name: &str) -> Result<Self> {
        let main_path = project_root.join(format!("{project_name}.typ"));
        Self::from_root_and_main(project_root.to_path_buf(), &main_path)
    }

    /// Creates a world by deriving root from `template_path`.
    /// Convenience constructor for the CLI path that only has a single path.
    pub fn from_template_path(template_path: &Path) -> Result<Self> {
        let root = template_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("template path has no parent"))?
            .to_path_buf();
        Self::from_root_and_main(root, template_path)
    }

    fn from_root_and_main(root: PathBuf, main_path: &Path) -> Result<Self> {
        let fonts = Fonts::searcher().search();

        let vpath = VirtualPath::within_root(main_path, &root)
            .ok_or_else(|| anyhow::anyhow!("template path is not within root"))?;
        let main_id = FileId::new(None, vpath);

        Ok(Self {
            root,
            main_id,
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            slots: Mutex::new(HashMap::new()),
            revision: AtomicU32::new(0),
            #[cfg(test)]
            reads: AtomicU32::new(0),
        })
    }

    /// Must be called before each compile.
    ///
    /// 1. Increments the revision counter.
    /// 2. Checks every cached slot against `(mtime, len)` on disk; stale slots
    ///    are cleared so the next `source()` / `file()` call re-reads them.
    /// 3. Removes slots not accessed in the last [`EVICT_MAX_AGE`] compiles.
    /// 4. Calls `comemo::evict(EVICT_MAX_AGE)` to trim the memo cache.
    pub fn reload(&mut self) -> Result<()> {
        let rev = self.revision.fetch_add(1, Ordering::Relaxed) + 1;

        let mut slots = self.slots.lock().unwrap();
        for (id, slot) in slots.iter_mut() {
            let path = self.root.join(id.vpath().as_rootless_path());
            if let Ok(meta) = fs::metadata(&path) {
                let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let len = meta.len();
                if slot.is_stale(mtime, len) {
                    slot.invalidate(mtime, len);
                }
            }
        }

        let cutoff = rev.saturating_sub(EVICT_MAX_AGE as u32);
        slots.retain(|_, slot| slot.last_accessed >= cutoff);
        drop(slots);

        comemo::evict(EVICT_MAX_AGE);
        Ok(())
    }

    /// Compiles the main document and returns a [`PagedDocument`].
    pub fn compile_document(&self) -> Result<PagedDocument> {
        let result = typst::compile(self);

        for w in &result.warnings {
            tracing::warn!("typst: {:?}", w);
        }

        result.output.map_err(|errors| {
            let msg = errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::anyhow!("Typst compilation failed:\n{msg}")
        })
    }

    fn path_for_id(&self, id: FileId) -> PathBuf {
        self.root.join(id.vpath().as_rootless_path())
    }

    fn load_source(&self, id: FileId) -> FileResult<Source> {
        let path = self.path_for_id(id);
        let mut slots = self.slots.lock().unwrap();
        let slot = self.read_or_insert_slot(id, &path, &mut slots);

        if slot.source.get().is_none() {
            #[cfg(test)]
            self.reads.fetch_add(1, Ordering::Relaxed);

            let text = fs::read_to_string(&path).map_err(|_| FileError::NotFound(path.clone()))?;
            let _ = slot.source.set(Source::new(id, text));
        }

        Ok(slot.source.get().unwrap().clone())
    }

    fn load_bytes(&self, id: FileId) -> FileResult<Bytes> {
        let path = self.path_for_id(id);
        let mut slots = self.slots.lock().unwrap();
        let slot = self.read_or_insert_slot(id, &path, &mut slots);

        if slot.bytes.get().is_none() {
            #[cfg(test)]
            self.reads.fetch_add(1, Ordering::Relaxed);

            let data = fs::read(&path).map_err(|_| FileError::NotFound(path.clone()))?;
            let _ = slot.bytes.set(Bytes::new(data));
        }

        Ok(slot.bytes.get().unwrap().clone())
    }

    fn read_or_insert_slot<'a>(
        &self,
        id: FileId,
        path: &PathBuf,
        slots: &'a mut HashMap<FileId, FileSlot>,
    ) -> &'a mut FileSlot {
        let slot = slots.entry(id).or_insert_with(|| {
            let (mtime, len) = fs::metadata(path)
                .map(|m| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()))
                .unwrap_or((SystemTime::UNIX_EPOCH, 0));
            FileSlot::new(mtime, len)
        });

        let rev = self.revision.load(Ordering::Relaxed);
        slot.last_accessed = rev;

        slot
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.load_source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.load_bytes(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).and_then(|slot| slot.get())
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn make_world(temp: &TempDir, name: &str, content: &str) -> TypstWorld {
        fs::write(temp.path().join(format!("{name}.typ")), content).unwrap();
        TypstWorld::new(temp.path(), name).unwrap()
    }

    #[test]
    fn new_initializes_empty_slots() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("t.typ"), "Hello").unwrap();
        let world = TypstWorld::new(temp.path(), "t").unwrap();
        assert!(world.slots.lock().unwrap().is_empty());
        assert_eq!(world.revision.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn compile_document_succeeds_on_minimal_template() {
        let temp = TempDir::new().unwrap();
        let mut world = make_world(&temp, "doc", "= Hello World");
        world.reload().unwrap();
        let doc = world.compile_document();
        assert!(doc.is_ok(), "{:?}", doc.err());
        assert_eq!(doc.unwrap().pages.len(), 1);
    }

    #[test]
    fn reload_keeps_unchanged_slots() {
        let temp = TempDir::new().unwrap();
        let mut world = make_world(&temp, "doc", "= Stable");

        world.reload().unwrap();
        let _ = world.compile_document().unwrap();
        let reads_first = world.reads.load(Ordering::Relaxed);

        world.reload().unwrap();
        let _ = world.compile_document().unwrap();
        let reads_second = world.reads.load(Ordering::Relaxed);

        assert_eq!(
            reads_first, reads_second,
            "no new disk reads on unchanged files"
        );
    }

    #[test]
    fn reload_invalidates_changed_main() {
        let temp = TempDir::new().unwrap();
        let mut world = make_world(&temp, "doc", "= Version 1");

        world.reload().unwrap();
        let doc1 = world.compile_document().unwrap();
        assert_eq!(doc1.pages.len(), 1);

        // Longer content → different len even on 1-s mtime filesystems.
        fs::write(
            temp.path().join("doc.typ"),
            "= Version 2\n\n#pagebreak()\n= Page 2",
        )
        .unwrap();

        world.reload().unwrap();
        let doc2 = world.compile_document().unwrap();
        assert_eq!(doc2.pages.len(), 2, "should see new content after reload");
    }

    #[test]
    fn reload_invalidates_changed_image() {
        let temp = TempDir::new().unwrap();
        let img_path = temp.path().join("img.png");
        fs::write(&img_path, make_png(1)).unwrap();

        let typ = "#set page(width: 50mm, height: 50mm)\n#image(\"img.png\", width: 10mm)";
        let mut world = make_world(&temp, "doc", typ);

        world.reload().unwrap();
        let _ = world.compile_document().unwrap();
        let reads_first = world.reads.load(Ordering::Relaxed);

        // Replace image with a larger file → guaranteed different len so that
        // invalidation fires even on 1-s mtime filesystems.
        fs::write(&img_path, make_png(8)).unwrap();

        world.reload().unwrap();
        let _ = world.compile_document().unwrap();
        let reads_second = world.reads.load(Ordering::Relaxed);

        assert!(
            reads_second > reads_first,
            "image slot should have been re-read after replacement"
        );
    }

    #[test]
    #[ignore = "timing test; run with --ignored"]
    fn second_compile_is_faster_than_first() {
        let temp = TempDir::new().unwrap();
        let mut world = make_world(&temp, "doc", "= Benchmark");

        world.reload().unwrap();
        let t0 = std::time::Instant::now();
        let _ = world.compile_document().unwrap();
        let first = t0.elapsed();

        world.reload().unwrap();
        let t1 = std::time::Instant::now();
        let _ = world.compile_document().unwrap();
        let second = t1.elapsed();

        assert!(
            second * 3 < first,
            "second compile ({second:?}) should be ≥3× faster than first ({first:?})"
        );
    }

    /// Generates a valid N×N white PNG using the `image` crate.
    /// Larger `size` → larger file → guaranteed different `len` for invalidation tests.
    fn make_png(size: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(size, size, Rgb([255u8, 255, 255]));
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        bytes
    }
}
