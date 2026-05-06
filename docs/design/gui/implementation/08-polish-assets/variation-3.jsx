// Variation 3 — "Whisper" (centered, drop-to-add).
// Goal: visual impression of the photobook page is NOT disturbed by chrome.
// - The page floats unframed, perfectly centered on dark.
// - Below each page: a tiny mode dot + "P00", centered under the page.
//   Hover reveals mode pill + regenerate + delete (still centered).
// - Between pages: a centered DROP ZONE. Drag a photo onto it to create a
//   new page. No click handler — the only way to add a page is by dropping
//   an image onto this zone.

const V3CentralPanel = () => {
  const [hover, setHover] = React.useState(null);
  const [dragOver, setDragOver] = React.useState(null);
  const pages = [
    { mode: 'A', tiles: PAGE0_TILES,            aspect: '16 / 11', cover: true, width: 520 },
    { mode: 'A', tiles: PAGE1_TILES,            aspect: '3 / 4',                width: 340 },
    { mode: 'M', tiles: PAGE0_TILES.slice(0,2), aspect: '16 / 11',              width: 520 },
  ];
  return (
    <div style={{
      height: '100%', overflow: 'auto', padding: '36px 0 60px',
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 0,
      background: FB.bg,
    }}>
      {pages.map((p, i) => (
        <React.Fragment key={i}>
          <V3PageBlock
            idx={i} {...p}
            hovered={hover === i}
            onEnter={() => setHover(i)}
            onLeave={() => setHover(null)}
          />
          {i < pages.length - 1 && (
            <V3DropZone
              active={dragOver === i}
              onDragEnter={() => setDragOver(i)}
              onDragLeave={() => setDragOver(null)}
            />
          )}
        </React.Fragment>
      ))}
    </div>
  );
};

const V3PageBlock = ({ idx, mode, tiles, aspect, cover, width, hovered, onEnter, onLeave }) => (
  <div
    onMouseEnter={onEnter}
    onMouseLeave={onLeave}
    style={{
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10,
      padding: '4px 0',
    }}>
    <FbPage tiles={tiles} aspect={aspect} width={width} cover={cover} />
    <V3PageHud idx={idx} mode={mode} hovered={hovered} />
  </div>
);

const V3PageHud = ({ idx, mode, hovered }) => {
  const color = mode === 'A' ? FB.auto : FB.manual;
  return (
    <div style={{
      height: 24, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10,
      fontFamily: FB.mono, fontSize: 10.5, letterSpacing: 0.5,
      color: FB.textMute,
      transition: 'opacity .15s',
      opacity: hovered ? 1 : 0.55,
    }}>
      <span>P{String(idx).padStart(2,'0')}</span>
      <V3ModePill mode={mode} expanded={hovered} />
      <div style={{
        display: 'flex', gap: 4,
        opacity: hovered ? 1 : 0,
        transform: hovered ? 'translateX(0)' : 'translateX(-4px)',
        transition: 'opacity .15s, transform .15s',
        pointerEvents: hovered ? 'auto' : 'none',
      }}>
        <V3MiniBtn title="Regenerate">↻</V3MiniBtn>
        <V3MiniBtn title="Delete" danger>✕</V3MiniBtn>
      </div>
    </div>
  );
};

const V3ModePill = ({ mode, expanded }) => {
  const color = mode === 'A' ? FB.auto : FB.manual;
  return (
    <button style={{
      display: 'inline-flex', alignItems: 'center', gap: 6,
      height: 18, padding: expanded ? '0 8px' : 0,
      width: expanded ? 'auto' : 10,
      borderRadius: 999,
      background: expanded ? `${color}22` : color,
      border: expanded ? `1px solid ${color}66` : 'none',
      color: color, fontFamily: FB.mono, fontSize: 10, fontWeight: 700,
      letterSpacing: 0.6, cursor: 'pointer',
      transition: 'width .18s, padding .18s, background .18s',
      overflow: 'hidden', whiteSpace: 'nowrap',
    }}>
      {expanded && (mode === 'A' ? '✦ AUTO' : '✋ MANUAL')}
    </button>
  );
};

const V3MiniBtn = ({ children, danger, title }) => (
  <button title={title} style={{
    width: 22, height: 22, padding: 0, borderRadius: 4,
    background: 'transparent', border: `1px solid ${FB.stroke}`,
    color: danger ? '#d97777' : FB.textDim, cursor: 'pointer',
    fontSize: 11, lineHeight: 1,
  }}>{children}</button>
);

// DROP ZONE — replaces the old "+ page" click button.
// Drag an image from the filmstrip onto this band to create a new page
// containing that image. No click action.
const V3DropZone = ({ active, onDragEnter, onDragLeave }) => (
  <div style={{
    width: '100%', display: 'flex', justifyContent: 'center',
    margin: '14px 0',
  }}>
    <div
      onDragEnter={(e) => { e.preventDefault(); onDragEnter(); }}
      onDragOver={(e) => e.preventDefault()}
      onDragLeave={onDragLeave}
      onDrop={(e) => { e.preventDefault(); onDragLeave(); }}
      style={{
        width: 520, height: 44,
        border: `1.5px dashed ${active ? FB.accent : FB.stroke}`,
        borderRadius: 6,
        background: active ? `${FB.accent}14` : 'transparent',
        color: active ? FB.accent : FB.textMute,
        fontFamily: FB.ui, fontSize: 11, letterSpacing: 0.3,
        display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
        transition: 'border-color .15s, background .15s, color .15s',
      }}>
      <DropIcon color={active ? FB.accent : FB.textMute} />
      <span>{active ? 'Release to add new page' : 'Drop a photo here to add a new page'}</span>
    </div>
  </div>
);

const DropIcon = ({ color }) => (
  <svg width="16" height="14" viewBox="0 0 16 14" fill="none">
    <rect x="1" y="1" width="14" height="12" rx="1" stroke={color} strokeWidth="1.2" strokeDasharray="2 2" />
    <circle cx="5" cy="5.5" r="1.2" fill={color} />
    <path d="M2 11 L6 7 L9 9.5 L12 6 L14 8.5 V12 H2 Z" fill={color} opacity="0.55" />
  </svg>
);

window.V3CentralPanel = V3CentralPanel;
