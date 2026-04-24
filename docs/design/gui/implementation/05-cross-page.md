# Phase 5: Cross-Page + Neue Seiten

**Ziel**: Vollständige Drag-Operationen über Seitengrenzen, [+]-Platzhalter
zwischen Seiten für Neue-Seite-Drops, restliche Hotkeys aus dem UX-Konzept.

**Voraussetzung**: Phase 4 abgeschlossen. Drei-Panel-Layout läuft, Cross-Page
Move für einzelne Slots funktioniert bereits (`input_handler::dispatch_move`
und `dispatch_swap` sind page-agnostisch). Cross-Page Swap sowie
Multi-Slot-Swap emittiert die GUI aber noch nicht.

---

## Status quo (Referenzpunkte im Code)

- `gui/app/input_handler.rs:271-294` (`dispatch_move`) — Cross-Page-Move mit
  Multi-Selection steht bereits. Einziger Input: `hovered_slot` oder
  `effective_page`.
- `gui/app/input_handler.rs:297-313` (`dispatch_swap`) — **nur Single-Slot**,
  cross-page-tauglich aber ohne Multi-Selection.
- `gui/background/commands.rs:99-117` (`run_swap_slots`) — akzeptiert bereits
  `dst_page != src_page`. Muss für Ranges erweitert werden.
- `gui/state/hover.rs` — `HoveredTarget` deckt `Page`, `NavPage`, `PoolItem`
  ab. Ein neuer Drop-Zone-Typ für `[+]` muss hier angehängt werden.
- Lib: `execute_move(..., PageMoveCmd::Move { src, dst: DstMove::NewPageAfter(p) })`
  existiert (`src/commands/page/move_cmd.rs:122-140`) und kümmert sich um
  Insert + Renumbering + leere-Seiten-Löschung.
- Lib: `execute_move(..., PageMoveCmd::Swap { left: Src::Slots{..}, right: DstSwap::Slots{..} })`
  unterstützt Multi-Slot (`SlotExpr`) und Cross-Page (siehe
  `src/commands/page/move_cmd.rs:229-319`). **Voraussetzung**: jede Seite
  nutzt eine **zusammenhängende** Slot-Range (`SlotExpr::from_range` oder
  `SlotExpr::single`) — nicht-kontinuierliche Listen werden mit
  `ValidationError::SwapNonContiguous` abgelehnt. Die GUI muss das
  respektieren (siehe 5.2.3).

---

## 5.0 — Scaffolding: Task/Command/Hover-Erweiterung

Ein Vorbereitungs-Commit analog zu 4.0 — danach die Feature-Commits.

### 5.0.1 `HoveredTarget` um `NewPageSlot` erweitern

```rust
// gui/state/hover.rs
pub enum HoveredTarget {
    Page { page: usize, slot: Option<usize> },
    NavPage(usize),
    PoolItem(String),
    /// [+]-Platzhalter zwischen Seiten im Central-Panel.
    /// `after_page = i` ⇒ Drop erzeugt eine neue Seite zwischen `i` und `i+1`.
    /// `after_page = num_pages - 1` ist zulässig (Seite ans Ende anhängen).
    NewPageSlot { after_page: usize },
}
```

Neue Helper-Methoden (ersetzen bestehende `page_idx`, `central_page`, `slot`
nicht — diese dürfen bei `NewPageSlot` weiterhin `None` liefern, damit
bestehender Code nicht versehentlich die `[+]`-Zone als Zielseite
missinterpretiert):

```rust
impl HoveredTarget {
    pub fn new_page_after(&self) -> Option<usize> {
        match self { Self::NewPageSlot { after_page } => Some(*after_page), _ => None }
    }
}
```

### 5.0.2 `PendingCommand` erweitern

```rust
// gui/app/pending.rs
pub enum PendingCommand {
    // … bestehende Varianten …

    /// Slots auf neuer Seite direkt nach `after_page` platzieren
    /// (= `page move {src_page}:{src_slots} to {after_page}+`).
    MoveToNewPage {
        src_page: usize,
        src_slots: Vec<usize>,
        after_page: usize,
    },

    /// Cross-Page Swap mit Slot-Range(s). Beide Seiten nutzen **eine**
    /// zusammenhängende Range (`src_slots` bzw. `dst_slots` sind bereits
    /// aufsteigend sortiert und lückenlos — Aufrufer muss das sicherstellen,
    /// sonst bricht die Lib mit `SwapNonContiguous` ab).
    SwapRange {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
        dst_slots: Vec<usize>,
    },

    /// Selektierte Slots unplacen (Delete-Taste, Drop auf Pool).
    Unplace { page: usize, slots: Vec<usize> },

    /// Selektierte Seiten neu bauen (R-Taste, nur Auto-Pages).
    RebuildPages { pages: Vec<usize> },

    /// Inkrementeller Build (Ctrl+B) bzw. Release-Build (Ctrl+Shift+B).
    Build { release: bool },
}
```

`PendingCommand::Swap` (single-slot) bleibt für Same-Page-Swaps ohne
Selektion. `SwapRange` ist der neue Pfad bei Multi-Selection-Drag — siehe
5.2.

### 5.0.3 `BackgroundTask` erweitern

```rust
// gui/task.rs
pub enum BackgroundTask {
    // … bestehende Varianten …

    MoveToNewPage {
        src_page: usize,
        src_slots: Vec<usize>,
        after_page: usize,
        pixel_per_pt: f32,
    },
    SwapRange {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
        dst_slots: Vec<usize>,
        pixel_per_pt: f32,
    },
    Unplace {
        page: usize,
        slots: Vec<usize>,
        pixel_per_pt: f32,
    },
    RebuildPages {
        pages: Vec<usize>,
        pixel_per_pt: f32,
    },
    Build {
        release: bool,
        pixel_per_pt: f32,
    },
}
```

### 5.0.4 Worker-Dispatch für neue Tasks

Neue Match-Arms in `gui/background.rs`, Delegierung an Helper in
`gui/background/commands.rs` nach bekanntem Muster (wie `run_swap_slots`):

```rust
// gui/background/commands.rs
pub(super) fn run_move_to_new_page(
    src_page: usize,
    src_slots: Vec<usize>,
    after_page: usize,
    rctx: &mut super::RenderCtx<'_>,
) {
    let cmd = PageMoveCmd::Move {
        src: Src::Slots {
            page: src_page as u32,
            slots: SlotExpr::from_list(src_slots.iter().map(|&s| s as u32).collect()),
        },
        dst: DstMove::NewPageAfter(after_page as u32),
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_swap_range(
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
    dst_slots: Vec<usize>,
    rctx: &mut super::RenderCtx<'_>,
) {
    let (s0, s1) = (*src_slots.first().unwrap(), *src_slots.last().unwrap());
    let (d0, d1) = (*dst_slots.first().unwrap(), *dst_slots.last().unwrap());
    let cmd = PageMoveCmd::Swap {
        left: Src::Slots {
            page: src_page as u32,
            slots: if s0 == s1 { SlotExpr::single(s0 as u32) }
                   else { SlotExpr::from_range(s0 as u32, s1 as u32) },
        },
        right: DstSwap::Slots {
            page: dst_page as u32,
            slots: if d0 == d1 { SlotExpr::single(d0 as u32) }
                   else { SlotExpr::from_range(d0 as u32, d1 as u32) },
        },
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_unplace(
    page: usize,
    slots: Vec<usize>,
    rctx: &mut super::RenderCtx<'_>,
) {
    // Unplace == page move ... -> out. Wir nutzen execute_unplace direkt,
    // weil das `pages_deleted` sauber liefert.
    let slot_expr = SlotExpr::from_list(slots.iter().map(|&s| s as u32).collect());
    match fotobuch::commands::unplace::execute_unplace(rctx.project_root, page as u32, slot_expr) {
        Err(e) => { let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(e.to_string())); }
        Ok(out) => finish_with_build(out, rctx),
    }
}

pub(super) fn run_rebuild_pages(
    pages: Vec<usize>,
    rctx: &mut super::RenderCtx<'_>,
) {
    // `rebuild --page N` nacheinander — Pages können einzeln manual sein und
    // werden dann als `CommandFailed` zurückgegeben (nicht fatal).
    let mut dirty: Vec<usize> = vec![];
    let mut new_state: Option<Box<fotobuch::dto_models::ProjectState>> = None;
    for p in pages {
        match fotobuch::commands::rebuild::rebuild(
            rctx.project_root,
            fotobuch::commands::rebuild::RebuildScope::SinglePage(p),
        ) {
            Err(e) => {
                let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(format!(
                    "rebuild page {p}: {e}"
                )));
            }
            Ok(out) => {
                if !dirty.contains(&p) { dirty.push(p); }
                if let Some(s) = out.changed_state { new_state = Some(Box::new(s)); }
            }
        }
    }
    super::render::send_command_done(new_state.map(|b| *b), dirty, rctx);
}

pub(super) fn run_build(release: bool, rctx: &mut super::RenderCtx<'_>) {
    let cfg = if release {
        fotobuch::commands::build::BuildConfig { release: true, force: false, pages: None }
    } else {
        fotobuch::commands::build::BuildConfig { release: false, force: false, pages: None }
    };
    match fotobuch::commands::build::build(rctx.project_root, &cfg) {
        Err(e) => { let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(e.to_string())); }
        Ok(out) => super::render::send_command_done(
            out.changed_state, out.result.pages_rebuilt, rctx,
        ),
    }
}
```

`finish_with_build` ist die in `run_page_command` bereits vorhandene Kette
(`execute_*` → `build()` → `CommandDone`) — extrahieren als private
Hilfsfunktion in `commands.rs`, damit Move/Swap/Unplace dieselbe Pipeline
fahren.

### 5.0.5 Commit

`feat(gui): phase 5 scaffolding — new page hover target, pending commands, tasks`

Tests (rein logisch):
- `pending::hover_new_page_after_returns_some`.
- `pending::swap_range_emits_expected_cmd_variant`.
- `background::run_swap_range_builds_single_slot_when_range_len_1` (prüft,
  dass `SlotExpr::single` gewählt wird, wenn `first == last`).

---

## 5.1 — [+]-Platzhalter zwischen Seiten

### 5.1.1 Layout in `draw_pages.rs`

Zwischen jedem Paar aufeinanderfolgender Seiten (und **nach** der letzten
Seite) wird ein schmales horizontales Rect gezeichnet. Inhalt lebt in neuem
Submodul `gui/app/widgets/central_panel/draw_new_page_slot.rs`:

```rust
pub(super) const NEW_PAGE_SLOT_HEIGHT_PT: f32 = 14.0;
pub(super) const NEW_PAGE_SLOT_MARGIN_PT: f32 = 4.0; // vertikaler Abstand

pub(super) fn draw(
    ui: &mut egui::Ui,
    after_page: usize,
    interaction: &InteractionState,
) -> (egui::Rect, bool /* hovered */) {
    let desired = egui::vec2(ui.available_width(), NEW_PAGE_SLOT_HEIGHT_PT);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::hover());

    let drag_active = !matches!(interaction.drag.active, ActiveDrag::Idle);
    let hovered = resp.hovered();

    // Alpha-Regel:
    //  kein Drag, kein Hover → sehr dezent (alpha 20)
    //  Drag aktiv, kein Hover → sichtbar (alpha 80)
    //  Hover (egal ob Drag)  → leuchtend (alpha 180), + Plus-Icon
    let alpha: u8 = match (drag_active, hovered) {
        (_, true)    => 180,
        (true, _)    => 80,
        _            => 20,
    };
    ui.painter().rect_filled(
        rect, 3.0,
        egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha),
    );
    if hovered {
        let icon_color = egui::Color32::from_rgb(255, 255, 255);
        ui.painter().text(
            rect.center(), egui::Align2::CENTER_CENTER, "+",
            egui::FontId::proportional(NEW_PAGE_SLOT_HEIGHT_PT - 2.0),
            icon_color,
        );
    }
    let _ = after_page;
    (rect, hovered)
}
```

### 5.1.2 Einhängen in die Scroll-Schleife

In `draw_pages::draw_pages` (gui/app/widgets/central_panel/draw_pages.rs,
Zeile 31-53) erweitern: zwischen je zwei Pages und nach der letzten Page
wird `draw_new_page_slot::draw` gerufen und der Hover in dasselbe
`hovered: Option<HoveredTarget>` geschrieben, wenn noch keiner gesetzt
ist:

```rust
for i in 0..num_pages {
    // bestehende draw_page-Logik für Seite i …

    // Platzhalter NACH Seite i — auch nach der letzten Seite.
    let (slot_rect, slot_hovered) = draw_new_page_slot::draw(ui, i, interaction);
    if hovered.is_none() && slot_hovered {
        hovered = Some(HoveredTarget::NewPageSlot { after_page: i });
    }
    let _ = slot_rect; // aktuell kein Scroll-Target nötig
}
```

Reihenfolge der Hit-Test-Prio (siehe `docs/design/gui/03-cli-gui-mapping.md`
§ Drop-Target-Auflösung): `[+]`-Slot hat Vorrang vor angrenzender Seite.
Weil `[+]` erst nach `draw_page` gezeichnet wird aber den Hover-Early-Exit
(`hovered.is_none()`) nutzt, kippt die Prio zur angrenzenden Seite. Fix:
Seite schreibt `hovered` nur dann, wenn **kein Cursor über einem
benachbarten [+]-Slot** ist. Einfachste Umsetzung: Pro Frame erst alle
[+]-Rects sammeln, dann in der eigentlichen Seiten-Schleife vor dem
Hover-Write prüfen, ob `ctx.pointer_hover_pos()` in einem davon liegt. Die
Rects sind bereits schmal — falsch-positive Seiten-Hovers unterhalb des
Plus sind selten; dennoch lohnt der Early-Check, damit Drop-Target und
visuelles Highlight übereinstimmen.

### 5.1.3 Drop auf [+] im Input-Handler

`complete_slot_drag` (gui/app/input_handler.rs:159) bekommt einen
zusätzlichen Arm **vor** dem bestehenden `match (hovered_slot, mode)`:

```rust
fn complete_slot_drag(
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
) {
    // NEU: Drop auf [+]-Platzhalter → Move auf neue Seite.
    if let Some(after) = interaction.hovered.as_ref().and_then(HoveredTarget::new_page_after) {
        let src_slots = selection_slots_for(interaction, src_page, src_slot);
        cmds.insert(PendingCommand::MoveToNewPage {
            src_page,
            src_slots,
            after_page: after,
        });
        return;
    }
    // … bestehende Logik (Swap / Move auf bestehende Seite) …
}
```

`selection_slots_for` ist der bestehende Snippet aus `dispatch_move`
(gui/app/input_handler.rs:278-287), herausgezogen als private Funktion, damit
`MoveToNewPage`, `Move` und der neue `SwapRange` die gleiche Regel nutzen:

```rust
fn selection_slots_for(interaction: &InteractionState, src_page: usize, src_slot: usize) -> Vec<usize> {
    if interaction.selections.slots.is_selected(src_page, src_slot) {
        match &interaction.selections.slots {
            SlotSelection::OnPage { page, slots, .. } if *page == src_page =>
                slots.iter().copied().collect(),
            _ => vec![src_slot],
        }
    } else {
        vec![src_slot]
    }
}
```

### 5.1.4 Leere Seiten

Phase-5-seitig nichts zu tun. `execute_move(..., DstMove::NewPageAfter(p))`
ruft intern `delete_empty_pages` (siehe `src/commands/page/move_cmd.rs:172`)
— eine vollständig entleerte Quellseite verschwindet automatisch, und die
Pipeline in `run_page_command` füttert die daraus resultierenden
`pages_deleted` plus `pages_inserted` in die UI. Nach `CommandDone` ruft
`drain_results` bereits `resize_page_vecs` auf (siehe Phase 3.3).

### 5.1.5 Commit + Tests

`feat(gui): insertion placeholders between pages trigger new-page move`

- `central_panel::new_page_slot_hover_returns_target` — fakes den Cursor
  über dem `[+]`-Rect und prüft `HoveredTarget::NewPageSlot`.
- `input_handler::drop_on_new_page_slot_emits_move_to_new_page` — Drag mit
  Selektion `{1, 2}` + Hover `NewPageSlot { after_page: 3 }` → genau ein
  `PendingCommand::MoveToNewPage { src_page, src_slots: [1,2], after_page: 3 }`.

---

## 5.2 — Cross-Page Drag mit Multi-Selektion

### 5.2.1 Cross-Page Move (bereits da, nur Commit für Doku)

`dispatch_move` erlaubt cross-page mit Multi-Selection bereits
(input_handler.rs:271). Kein Code-Change. Phase 5.2 zieht nur den Test
nach, der bisher fehlt:

- `input_handler::cross_page_move_with_selection_moves_all_selected_slots`:
  Selektion `{0, 2}` auf Seite 3, Drag über Slot auf Seite 7, `M` gedrückt
  → `PendingCommand::Move { src_page: 3, src_slots: [0,2], dst_page: 7 }`.

### 5.2.2 Cross-Page Swap mit Range

Bisher emittiert `dispatch_swap` (input_handler.rs:297) nur
`PendingCommand::Swap` mit Single-Slot. Die Lib unterstützt Multi-Slot-Swap
via `SlotExpr` mit einer zusammenhängenden Range. Damit die GUI das nutzt,
gilt:

1. **Quelle**: `src_slots` kommt aus der Selektion (oder Single-Slot wenn
   gedraggter Slot nicht Teil der Selektion ist) — identisch zu
   `selection_slots_for`.
2. **Ziel**: `dst_slots` hat die **gleiche Anzahl** wie `src_slots` und
   beginnt beim gehoverten `dst_slot`:

   ```rust
   fn compute_dst_range(dst_slot: usize, count: usize, layout_dst: &LayoutPage)
       -> Option<Vec<usize>>
   {
       let end_excl = dst_slot + count;
       if end_excl > layout_dst.slots.len() { return None; }
       Some((dst_slot..end_excl).collect())
   }
   ```

   Wenn `end_excl` über die Ziel-Seitenslot-Anzahl hinausragt, wird der
   Swap nicht emittiert (silent no-op; visuell schon vorher durch rotes
   Overlay angekündigt, siehe 5.2.4).

### 5.2.3 Dispatch-Logik aktualisieren

`complete_slot_drag` verzweigt jetzt nach Selection-Größe:

```rust
match (hovered_slot, interaction.drag.mode) {
    (Some((dst_page, dst_slot)), DragMode::Swap) => {
        let src_slots = selection_slots_for(interaction, src_page, src_slot);
        if src_slots.len() == 1 {
            dispatch_swap(cmds, src_page, src_slots[0], dst_page, dst_slot);
        } else {
            // Multi-Select-Swap: Range beim Ziel-Slot fortsetzen.
            let layout_dst = match data.project.layout.get(dst_page) {
                Some(p) => p, None => return,
            };
            if !is_contiguous(&src_slots) { return; }  // Lib würde SwapNonContiguous werfen
            if let Some(dst_slots) = compute_dst_range(dst_slot, src_slots.len(), layout_dst) {
                cmds.insert(PendingCommand::SwapRange {
                    src_page, src_slots, dst_page, dst_slots,
                });
            }
        }
    }
    // Move / Move-ohne-Slot / Swap-ohne-Slot unverändert …
}
```

`is_contiguous(slots: &[usize]) -> bool` ist `slots.windows(2).all(|w| w[1]
== w[0] + 1)`. `selection_slots_for` liefert bereits aufsteigend sortiert
(BTreeSet).

Signatur von `complete_slot_drag` wächst um `data: &DataState` — Aufrufer
`handle_drag_complete` reicht das einfach durch (`handle` hat
`data: &mut DataState` in Phase 5 ohnehin nötig für Ctrl+G und R).

### 5.2.4 Visuelles Feedback bei Range-Swap

`draw_page` zeichnet die Zielslots (`dst_slot..dst_slot+count`) analog zur
bestehenden Ziel-Slot-Markierung. Bei Over-Range
(`dst_slot + count > slots.len()`) wird die **gesamte** potenzielle Range
bis zum Seitenende rot eingefärbt (Farbkonstante aus Phase 2 übernehmen:
`Color32::from_rgba_unmultiplied(220, 40, 40, 80)`), der nicht existente
Rest bekommt keine Markierung — klare Fehlersignalisierung.

### 5.2.5 Commits + Tests

1. `feat(gui): test coverage for cross-page move with multi-selection`
2. `feat(gui): cross-page range swap via SlotExpr`

Tests (rein logisch, kein egui):
- `input_handler::swap_range_uses_full_selection_when_dragged_slot_selected`.
- `input_handler::swap_range_noop_when_target_too_narrow`.
- `input_handler::swap_range_noop_when_selection_not_contiguous`.
- `input_handler::swap_falls_back_to_single_when_selection_is_one`.

---

## 5.3 — Hotkeys komplett

Alle fehlenden Hotkeys aus `docs/design/gui/01-ux-konzept.md` nachziehen.
Ein Commit pro Hotkey-Gruppe (nicht pro Taste), damit die Tests pro Feature
sinnvoll gebündelt bleiben.

### 5.3.1 `Delete` → Unplace

```rust
// gui/app/input_handler.rs
fn handle_delete(interaction: &mut InteractionState, ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) { return; }
    let SlotSelection::OnPage { page, slots, .. } = &interaction.selections.slots else { return; };
    if slots.is_empty() { return; }
    cmds.insert(PendingCommand::Unplace {
        page: *page,
        slots: slots.iter().copied().collect(),
    });
    // Selektion bleibt — wird erst in drain_results geleert (bestehendes Muster).
}
```

Aufruf in `handle()` **vor** `handle_click`, damit Delete nie fälschlich
eine Selektion verwirft.

Pool-Selektion + Delete wird in Phase 6 (Pool-Remove) gesondert behandelt.

### 5.3.2 `R` → Rebuild selektierter Seite(n)

```rust
fn handle_rebuild(interaction: &InteractionState, ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) { return; }
    // Aktive Seiten-Menge:
    //   1) aus Slot-Selektion → genau eine Seite
    //   2) sonst aus `hovered` (Central oder Nav)
    let pages: Vec<usize> = match &interaction.selections.slots {
        SlotSelection::OnPage { page, .. } => vec![*page],
        SlotSelection::None => match &interaction.hovered {
            Some(HoveredTarget::Page { page, .. }) | Some(HoveredTarget::NavPage(page)) => vec![*page],
            _ => return,
        },
    };
    cmds.insert(PendingCommand::RebuildPages { pages });
}
```

Manual-Pages liefern in der Lib `anyhow::bail!` (siehe
`src/commands/rebuild.rs:116-122`). Die Pipeline in
`run_rebuild_pages` fängt das als `CommandFailed` und zeigt später den
Toast an (Toasts selbst kommen in Phase 6.2).

### 5.3.3 `Ctrl+G` → Gehe zu Seite

Neues Popup-State-Feld:

```rust
// gui/state/viewport.rs (oder neues Submodul gui/state/goto.rs)
#[derive(Default)]
pub struct GotoPageDialog {
    pub open: bool,
    pub buffer: String, // rohe Eingabe, 0-basiert
}
```

Hängt an `InteractionState::goto: GotoPageDialog` (neues Feld).

Input-Handler:

```rust
fn handle_goto_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
        interaction.goto.open = !interaction.goto.open;
        if interaction.goto.open { interaction.goto.buffer.clear(); }
    }
}
```

Widget: `gui/app/widgets/goto_dialog.rs` — `egui::Window::new("Gehe zu
Seite").open(&mut state.interaction.goto.open).collapsible(false)`,
Single-Line `text_edit_singleline`. `Enter` parst `buffer` als `usize`,
setzt bei Erfolg `interaction.viewport.scroll.scroll_to_page = Some(idx)`
(bestehender Mechanismus aus Phase 4.1.4) und schließt das Fenster;
`Escape` schließt ohne Aktion. Ungültige Eingaben (kein `usize`, Index
außerhalb `layout.len()`) bleiben im Fenster und färben das Feld rot
(`ui.visuals_mut().override_text_color = Some(Color32::LIGHT_RED)` solange
`parse`+Range-Check scheitert).

### 5.3.4 `Home` / `End` → Erste / Letzte Seite

```rust
fn handle_home_end(interaction: &mut InteractionState, data: &DataState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
        interaction.viewport.scroll.scroll_to_page = Some(0);
    } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::End)) {
        let last = data.project.layout.len().saturating_sub(1);
        interaction.viewport.scroll.scroll_to_page = Some(last);
    }
}
```

### 5.3.5 `Ctrl+0` → Seitenbreite einpassen

`Viewport::fit_pending` existiert bereits (siehe `draw_pages.rs:58-74`) —
nur triggern:

```rust
fn handle_fit_width(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0)) {
        interaction.viewport.fit_pending = true;
    }
}
```

### 5.3.6 `Ctrl+B` / `Ctrl+Shift+B` → Build / Release-Build

```rust
fn handle_build(ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::B)) {
        cmds.insert(PendingCommand::Build { release: true });
        return;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::B)) {
        cmds.insert(PendingCommand::Build { release: false });
    }
}
```

Reihenfolge wichtig: erst `Ctrl+Shift+B` konsumieren (egui meldet Shift als
Teil der Modifier-Maske — `Ctrl+Shift+B` darf nicht von `Ctrl+B` mit
„extra Shift“ verschluckt werden, deshalb der frühe `return`).

Toolbar-Buttons `[Build]` / `[Release]` in
`gui/app/widgets/toolbar.rs` emittieren dieselben `PendingCommand::Build`.

### 5.3.7 `Ctrl+O` → Add-Dialog

Phase 5 liefert **nur** das Öffnen eines Modal-Placeholders (ohne
Funktionalität) und das Konsumieren der Taste, damit sie nicht mehr als
„Hotkey fehlt" auffällt. Die eigentliche Add-Integration (Filepicker,
Dialog, `commands::add::add`) ist Scope von Phase 6.0, zusammen mit
„OS-Drag von Dateien auf das Fenster".

```rust
fn handle_add_hotkey(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
        interaction.add_dialog.open = true;   // Stub in Phase 5
    }
}
```

`InteractionState::add_dialog: AddDialogState { open: bool }` — volle
Struktur in Phase 6.0. Phase-5-Widget zeigt nur ein Fenster mit
„Noch nicht implementiert" und einen Close-Button.

### 5.3.8 Commit-Aufteilung + Tests

1. `feat(gui): delete-key unplaces selected slots`
2. `feat(gui): r-key rebuilds active page(s)`
3. `feat(gui): ctrl+g go-to-page dialog`
4. `feat(gui): home/end scroll to first/last page`
5. `feat(gui): ctrl+0 fits page width`
6. `feat(gui): ctrl+b/ctrl+shift+b trigger build/release`
7. `feat(gui): ctrl+o opens add-dialog stub`

Tests (jeweils in `gui/app/input_handler.rs::tests` oder passendem
Submodul):
- `handle_delete_emits_unplace_with_selection_slots`.
- `handle_delete_noop_without_selection`.
- `handle_rebuild_uses_slot_selection_page_if_present`.
- `handle_rebuild_uses_hovered_page_when_no_selection`.
- `handle_home_end_scrolls_to_correct_page`.
- `handle_fit_width_sets_fit_pending`.
- `handle_build_hotkeys_distinguish_release`.
- `handle_goto_dialog_toggle_clears_buffer_on_open`.

---

## 5.4 Akzeptanzkriterien

- [ ] Zwischen je zwei Seiten sowie nach der letzten Seite erscheint im
      Central-Panel ein schmales blaues Rechteck (sehr dezent ohne Drag,
      heller bei aktivem Drag, leuchtend bei Hover).
- [ ] RMB-Drag auf ein `[+]`-Rect erzeugt eine neue Seite direkt nach
      `after_page` und verschiebt die Selektion (oder den einzeln
      gedraggten Slot) dorthin. Komplett entleerte Quellseiten
      verschwinden (durch Lib gehandhabt).
- [ ] Cross-Page Slot-Move mit Multi-Selektion verschiebt alle selektierten
      Slots auf die gehoverte Zielseite (via `dispatch_move` — bereits
      umgesetzt, Phase 5 fügt nur den Regressionstest hinzu).
- [ ] Cross-Page Slot-Swap mit Multi-Selektion (Default-Drag ohne M) tauscht
      eine zusammenhängende Slot-Range gegen eine gleich große Range am
      Ziel. Overrun wird durch ein rotes Overlay angekündigt und silent
      nicht emittiert.
- [ ] `Delete` unplaced die selektierten Slots (Multi ok). Leere Seite
      verschwindet.
- [ ] `R` rebuildet die aktive Seite (Slot-Selektion bevorzugt, sonst
      hovered Page). Manual-Pages liefern einen Fehler-Toast.
- [ ] `Ctrl+G` öffnet Popup; Enter mit gültigem 0-basierten Index scrollt
      dorthin, Escape schließt ohne Aktion.
- [ ] `Home` / `End` scrollen zur ersten / letzten Seite.
- [ ] `Ctrl+0` passt den Zoom so an, dass die breiteste Seite die
      Panel-Breite füllt.
- [ ] `Ctrl+B` triggert inkrementellen Build, `Ctrl+Shift+B` Release-Build.
- [ ] `Ctrl+O` öffnet einen Stub-Dialog (echte Add-Funktion in Phase 6).
- [ ] `cargo build --features gui` und `cargo clippy --features gui --
      -D warnings` sauber; `cargo fmt --check` sauber; `cargo test` grün.

## 5.5 Was NICHT in Phase 5 gehört

- Manual-Mode-Toggle `[A|M]`, Slot-Free-Positioning, Solver-Skip-Logik —
  Phase 6.1.
- Pool-Add über OS-Drag, Pool-Remove über Delete — Phase 6.0.
- Render-Cancellation, Zoom-Debouncing, Thumbnail-Only-Off-Screen —
  Phase 6.2.
- Toast-System (Fehler-Popups für `CommandFailed`) — Phase 6.2.
- Rechtsklick-Kontextmenü auf Slots / Pages / Gruppen — Phase 6.2.
- History-Dropdown (Undo-Liste), `fotobuch history`-Integration —
  nachgelagert.
- Drag inside Pool (Reordering von `PhotoFile`s) — nicht vorgesehen.

