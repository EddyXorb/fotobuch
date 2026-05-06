// Shared tokens and components used across all 4 variations.
// Style direction: flat, panel-heavy, slightly monospace — matches egui's
// native vibe (immediate-mode, no gradients, clear hit targets).

const FB = {
  // App chrome
  bg:        '#1f2024',          // body
  panel:     '#26282d',          // panels
  panelHi:   '#2e3036',          // hover / raised
  divider:   '#13141770',
  stroke:    '#3a3d44',
  strokeHi:  '#52565f',
  text:      '#e7e6e2',
  textDim:   '#9ea2ab',
  textMute:  '#6b6f78',
  // Accents — single warm photo-orange + a calm teal for state
  accent:    '#e08840',          // primary action / Move-mode
  accentSoft:'#e0884033',
  ok:        '#5fb56a',          // placed indicator
  warn:      '#d6a23a',
  auto:      '#6aa9ff',          // auto-mode (cool, "system")
  autoSoft:  '#6aa9ff22',
  manual:    '#c8b18a',          // manual-mode (warm, "hand")
  manualSoft:'#c8b18a22',
  // Page (the actual fotobuch page, on dark bg)
  paper:     '#f4f1ec',
  paperEdge: '#0008',
  // Type
  ui:        '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif',
  mono:      '"JetBrains Mono", "Fira Code", ui-monospace, "SF Mono", Menlo, Consolas, monospace',
  radius: 6,
};

// ---------- Tiny egui-flavored primitives ----------

const FbBtn = ({ children, primary, active, icon, style, ghost, ...p }) => (
  <button
    {...p}
    style={{
      display: 'inline-flex', alignItems: 'center', gap: 6,
      height: 26, padding: '0 10px',
      background: primary ? FB.accent : active ? FB.panelHi : ghost ? 'transparent' : FB.panel,
      color: primary ? '#1a1208' : FB.text,
      border: `1px solid ${primary ? FB.accent : ghost ? 'transparent' : FB.stroke}`,
      borderRadius: 5,
      fontFamily: FB.ui, fontSize: 12,
      fontWeight: primary ? 600 : 500,
      cursor: 'pointer',
      whiteSpace: 'nowrap',
      ...style,
    }}>
    {icon && <span style={{ fontSize: 13, opacity: .9 }}>{icon}</span>}
    {children}
  </button>
);

// ---------- Top toolbar ----------

const FbToolbar = () => (
  <div style={{
    display: 'flex', alignItems: 'center', gap: 6, padding: '8px 10px',
    background: FB.panel, borderBottom: `1px solid ${FB.divider}`,
    fontFamily: FB.ui, fontSize: 12, color: FB.text,
  }}>
    <div style={{
      display: 'flex', alignItems: 'center', gap: 6, height: 26, padding: '0 10px',
      background: FB.bg, border: `1px solid ${FB.stroke}`, borderRadius: 5, minWidth: 130,
    }}>
      <span>my-fotobuch</span>
      <span style={{ marginLeft: 'auto', opacity: .6 }}>▾</span>
    </div>
    <div style={{ width: 1, height: 18, background: FB.stroke, margin: '0 4px' }} />
    <FbBtn>Add</FbBtn>
    <FbBtn>Place</FbBtn>
    <FbBtn>Rebuild</FbBtn>
    <FbBtn>Release</FbBtn>
    <div style={{ width: 1, height: 18, background: FB.stroke, margin: '0 4px' }} />
    <FbBtn ghost icon="↶" />
    <FbBtn ghost icon="↷" />
    <FbBtn icon="◷">History</FbBtn>
    <FbBtn icon="⚙">Config</FbBtn>
    <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
      <FbBtn>☐ Slot info</FbBtn>
      <FbBtn icon="⇄">Swap</FbBtn>
      <FbBtn primary icon="→">Move</FbBtn>
    </div>
  </div>
);

// ---------- Left filmstrip (photo library) ----------

const PHOTOS = [
  ['#7a8c5b', 'lynx'],     ['#a7b489', 'lily'],     ['#6b6f78', 'cat'],
  ['#9c8866', 'puffin'],   ['#3d4a2b', 'macarons'], ['#a23a3a', 'flower'],
  ['#252830', 'city'],     ['#3a5a3a', 'peacock'],  ['#5fb56a', 'frog'],
  ['#e8d4a8', 'milk'],     ['#1a1d24', 'horse'],    ['#7a6048', 'parrot'],
  ['#5a8c7a', 'beach'],    ['#a8b5c4', 'mountain'], ['#c84030', 'tomato'],
  ['#7a9a4a', 'leaf'],     ['#4a7caf', 'water'],    ['#8c5a3a', 'wood'],
  ['#3a3a3a', 'eye'],      ['#6a4a2a', 'frame'],    ['#9a7a5a', 'figure'],
];

const FbFilmstrip = ({ width = 290 }) => (
  <div style={{
    width, background: FB.panel, borderRight: `1px solid ${FB.divider}`,
    overflow: 'hidden', display: 'flex', flexDirection: 'column',
    fontFamily: FB.ui, fontSize: 11, color: FB.textDim,
  }}>
    {PHOTOS.map(([c, name], i) => (
      <div key={i} style={{
        display: 'flex', alignItems: 'center', gap: 8, padding: '3px 8px',
        borderBottom: `1px solid ${FB.divider}`, height: 26, flex: '0 0 auto',
      }}>
        <div style={{
          width: 30, height: 20, background: c, borderRadius: 2, flex: '0 0 auto',
          boxShadow: 'inset 0 0 0 1px #0006',
        }} />
        <div style={{
          flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
          fontFamily: FB.mono, fontSize: 10.5, color: FB.textDim,
        }}>image-from-rawpixel-id-{6035000 + i * 137}-jpeg.jpg</div>
        <div style={{
          width: 7, height: 7, borderRadius: 7, background: FB.ok, flex: '0 0 auto',
          boxShadow: `0 0 6px ${FB.ok}80`,
        }} />
      </div>
    ))}
  </div>
);

// ---------- Right rail (page thumbnails) ----------

const FbRightRail = ({ width = 130, current = 0 }) => (
  <div style={{
    width, background: FB.panel, borderLeft: `1px solid ${FB.divider}`,
    padding: '10px 12px', display: 'flex', flexDirection: 'column', gap: 14,
    fontFamily: FB.ui, fontSize: 11, color: FB.textDim, overflow: 'hidden',
  }}>
    {[0,1,2,3,4].map(i => (
      <div key={i} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        <div style={{
          aspectRatio: '1 / 1', background: FB.paper, borderRadius: 2,
          padding: 5, position: 'relative',
          outline: i === current ? `2px solid ${FB.accent}` : 'none',
          outlineOffset: 2,
        }}>
          <MiniPage variant={i} />
        </div>
        <div style={{ fontFamily: FB.mono, fontSize: 10, opacity: .7 }}>P{i}</div>
      </div>
    ))}
  </div>
);

// Tiny page thumbnail (varied collage shapes)
const MiniPage = ({ variant = 0 }) => {
  const layouts = [
    [[0,0,50,100,'#a23a3a'],[50,0,50,100,'#7a8c5b']],
    [[0,0,33,50,'#3a3a3a'],[33,0,34,50,'#4a7caf'],[67,0,33,50,'#e8d4a8'],
     [0,50,50,50,'#7a6048'],[50,50,50,50,'#c84030']],
    [[0,0,100,55,'#252830'],[0,55,50,45,'#5fb56a']],
    [[0,0,50,33,'#a7b489'],[50,0,50,33,'#6b6f78'],[0,33,33,34,'#a23a3a'],
     [33,33,34,34,'#252830'],[67,33,33,34,'#3a5a3a'],[0,67,50,33,'#7a9a4a'],[50,67,50,33,'#4a7caf']],
    [[0,0,50,50,'#c84030'],[50,0,50,50,'#4a7caf'],[0,50,50,50,'#7a8c5b'],[50,50,50,50,'#9a7a5a']],
  ];
  const tiles = layouts[variant] || layouts[0];
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      {tiles.map(([x,y,w,h,c], i) => (
        <div key={i} style={{
          position: 'absolute', left: `${x}%`, top: `${y}%`, width: `${w}%`, height: `${h}%`,
          background: c, outline: '0.5px solid #0006',
        }} />
      ))}
    </div>
  );
};

// ---------- The fotobuch page (collage) — used inside the central panel ----------

const PAGE0_TILES = [
  // The "puffin / hill / cover" page from the screenshot, abstracted
  { x: 0, y: 0, w: 47, h: 100, c: '#1a1d24', label: 'puffin' },
  { x: 47, y: 0, w: 6, h: 100, c: FB.paper, label: '' },
  { x: 53, y: 0, w: 47, h: 100, c: '#7a8c5b', label: 'hill' },
];
const PAGE1_TILES = [
  { x: 5, y: 5, w: 28, h: 22, c: '#3a3a3a' },
  { x: 35, y: 5, w: 28, h: 22, c: '#4a7caf' },
  { x: 65, y: 5, w: 30, h: 22, c: '#e8d4a8' },
  { x: 5, y: 30, w: 28, h: 22, c: '#7a6048' },
  { x: 35, y: 30, w: 28, h: 22, c: '#a23a3a' },
  { x: 65, y: 30, w: 30, h: 22, c: '#5fb56a' },
  { x: 5, y: 55, w: 28, h: 22, c: '#7a9a4a' },
  { x: 35, y: 55, w: 28, h: 22, c: '#252830' },
  { x: 65, y: 55, w: 30, h: 22, c: '#9a7a5a' },
  { x: 5, y: 80, w: 28, h: 17, c: '#3a5a3a' },
  { x: 35, y: 80, w: 28, h: 17, c: '#1a1d24' },
  { x: 65, y: 80, w: 30, h: 17, c: '#7a6048' },
];

// width prop = on-screen width in px
const FbPage = ({ tiles = PAGE0_TILES, aspect = '16 / 11', width = 360, cover = false }) => (
  <div style={{
    width, aspectRatio: aspect, background: FB.paper,
    boxShadow: '0 1px 0 #fff2 inset, 0 6px 22px #0009, 0 0 0 1px #0006',
    position: 'relative', overflow: 'hidden',
  }}>
    {tiles.map((t, i) => (
      <div key={i} style={{
        position: 'absolute',
        left: `${t.x}%`, top: `${t.y}%`, width: `${t.w}%`, height: `${t.h}%`,
        background: t.c,
        boxShadow: t.c === FB.paper ? 'none' : 'inset 0 0 0 1px #0003',
      }} />
    ))}
    {cover && (
      <div style={{
        position: 'absolute', left: '50%', top: '50%',
        transform: 'translate(-50%, -50%) rotate(-90deg)',
        fontFamily: FB.ui, fontSize: Math.round(width * 0.022), color: '#3a2e22',
        whiteSpace: 'nowrap', letterSpacing: 0.3,
      }}>my-fotobuch</div>
    )}
  </div>
);

// ---------- Frame: full app shell with a swap-in central panel ----------

const FbFrame = ({ children, w = 1280, h = 880 }) => (
  <div style={{
    width: w, height: h, background: FB.bg, color: FB.text,
    display: 'flex', flexDirection: 'column', overflow: 'hidden',
    boxShadow: '0 30px 80px #0006',
  }}>
    <FbToolbar />
    <div style={{ display: 'flex', flex: 1, minHeight: 0 }}>
      <FbFilmstrip />
      <div style={{ flex: 1, minWidth: 0, position: 'relative', overflow: 'hidden' }}>
        {children}
      </div>
      <FbRightRail />
    </div>
    <div style={{
      padding: '6px 12px', background: FB.panel, borderTop: `1px solid ${FB.divider}`,
      fontFamily: FB.mono, fontSize: 11, color: FB.textDim,
      display: 'flex', gap: 14,
    }}>
      <span>Page –/8</span><span>·</span>
      <span>87 photos</span><span>·</span>
      <span>22 unplaced</span><span>·</span>
      <span>Sel: —</span><span>·</span>
      <span style={{ color: FB.accent }}>Move</span>
    </div>
  </div>
);

Object.assign(window, {
  FB, FbBtn, FbToolbar, FbFilmstrip, FbRightRail, FbPage, FbFrame, MiniPage,
  PAGE0_TILES, PAGE1_TILES,
});
