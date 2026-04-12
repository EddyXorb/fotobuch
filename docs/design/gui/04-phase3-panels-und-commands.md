# Phase 3: Drei-Panel-Layout + Command Pipeline

## Ziel

Die GUI bekommt das Drei-Panel-Layout (Foto-Pool, Hauptansicht, Seiten-Nav) und kann erstmals Lib-Commands ausführen (Unplace, Undo/Redo). Betroffene Seiten werden nach jedem Command automatisch neu gerendert.

## Modul-Refactoring

```
gui/app/
+   panels.rs             deklariert main_view, photo_pool, page_nav
+   panels/
+       main_view.rs      bisherige view.rs → hierher verschoben
+       photo_pool.rs     linkes SidePanel
+       page_nav.rs       rechtes SidePanel
```

`app/view.rs` wird zu `app/panels/main_view.rs`. `geometry.rs` bleibt in `app/`.

### Panel-Aufbau in `FotobuchApp::ui()`

```rust
SidePanel::left("photo_pool").resizable(true).default_width(200.0).show_inside(ui, |ui| {
    panels::photo_pool::show(ui, &self.state);
});
SidePanel::right("page_nav").resizable(true).default_width(100.0).show_inside(ui, |ui| {
    panels::page_nav::show(ui, &mut self.state);
});
CentralPanel::default().show_inside(ui, |ui| self.show_pages(ui));
```

## Foto-Pool (links)

- `ScrollArea::vertical` mit `CollapsingHeader` pro Gruppe
- Header-Text: `Gruppenname (placed/total)`
- Pro Foto: `ui.label(id)` — platzierte gedimmt + `[S.{page}]`-Badge, unplatzierte normal
- Alles aus `DerivedState` (vorhanden: `group_of_photo`, `placement_of_photo`, `unplaced_photos`)
- Keine Drag-Interaktion in Phase 3

## Seiten-Navigation (rechts)

- `ScrollArea::vertical`, ein Eintrag pro Seite
- Thumbnail: existierende `page_textures[i]` skaliert auf ~80px Breite (egui `Image::fit_to_exact_size`). Grauer Platzhalter solange `None`.
- Label `S.{i}` + `[A]`/`[M]`-Badge aus `layout[i].mode`
- Klick setzt `GuiState::scroll_target: Option<usize>` — Hauptansicht evaluiert dies im nächsten Frame via `ui.scroll_to_rect(page_rects[target])`
- Dafür muss `show_pages` die Seiten-Rects zwischenspeichern: `GuiState::page_rects: Vec<egui::Rect>`

## Command Pipeline

### task.rs erweitern

```rust
pub enum BackgroundTask {
    RenderPages { pages: Vec<usize>, pixel_per_pt: f32 },
    RunCommand(Box<dyn FnOnce() -> CommandDone + Send>),
}

pub enum CommandDone {
    PageMove(CommandOutput<PageMoveResult>),
    Undo(CommandOutput<UndoResult>),
    Failed(String),
}

pub enum BackgroundResult {
    PageRendered { page: RenderedPage, rasterize_duration: Duration, compile_duration: Duration },
    CommandDone(CommandDone),
    Error(String),
}
```

`CommandDone` nutzt direkt `CommandOutput<T>` aus der Lib — kein separates `state`-Feld nötig.

### Dirty Pages aus CommandDone

```rust
impl CommandDone {
    pub fn dirty_pages(&self, total: usize) -> Vec<usize> {
        match self {
            Self::PageMove(o) => o.result.pages_modified.iter()
                .chain(&o.result.pages_inserted)
                .map(|p| *p as usize).collect(),
            Self::Undo(_) => (0..total).collect(),  // State komplett neu
            Self::Failed(_) => vec![],
        }
    }

    pub fn take_state(&mut self) -> Option<ProjectState> {
        match self {
            Self::PageMove(o) => o.changed_state.take(),
            Self::Undo(o) => o.changed_state.take(),
            Self::Failed(_) => None,
        }
    }
}
```

### Worker-Thread (background.rs)

Neue Match-Arm im Task-Loop:

```rust
BackgroundTask::RunCommand(f) => {
    let done = f();
    let _ = result_tx.send(BackgroundResult::CommandDone(done));
}
```

Kein `world.reload()` hier — das passiert beim nächsten `RenderPages`.

### FotobuchApp erweitern

`project_root: PathBuf` als Feld hinzufügen (wird von `main.rs` übergeben). Commands bauen Closures, die `project_root.clone()` capturen.

### State-Update in drain_results

```rust
BackgroundResult::CommandDone(mut done) => {
    let dirty = done.dirty_pages(self.state.page_textures.len());
    if let Some(new_state) = done.take_state() {
        self.state.project_state = new_state;
        self.state.derived = DerivedState::rebuild(&self.state.project_state);
        self.state.page_textures.resize_with(
            self.state.project_state.layout.len(), || None,
        );
    }
    if !dirty.is_empty() {
        let _ = self.task_tx.send(BackgroundTask::RenderPages {
            pages: dirty,
            pixel_per_pt: self.state.base_pixel_per_pt * self.state.zoom,
        });
    }
}
```

## Erste Commands

| Trigger | Lib-Aufruf | Selection → Argumente |
|---------|-----------|----------------------|
| `Delete` | `execute_unplace(root, page, slots)` | `page as u32`, `SlotExpr::from_list(slots → Vec<u32>)` |
| `Ctrl+Z` | `undo(root, 1)` | keine |
| `Ctrl+Y` | `redo(root, 1)` | keine |

Alle drei werden in `handle_input` als `RunCommand`-Closure an den Worker geschickt. Nach dem Absenden: `Selection::clear()` (Unplace) bzw. keine UI-Aktion (Undo/Redo).

## Nicht in Phase 3

- Drag & Drop (Phase 4)
- Kontextmenüs
- Config-Panel
- Toolbar-Aktionen (Build, Release)
- Epoch-basierte Invalidierung (YAGNI bis Re-Render-Races auftreten)
- Blur/Spinner-Overlay für buildende Seiten
