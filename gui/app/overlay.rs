use crate::state::Timings;

pub fn show_timings_overlay(timings: &Timings, ctx: &egui::Context) {
    egui::Window::new("Timings [F2]")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            egui::Grid::new("timings_grid")
                .num_columns(2)
                .spacing([12.0, 2.0])
                .show(ui, |ui| {
                    ui.label("frame");
                    ui.label(format!("{}", timings.frame_cnt));
                    ui.end_row();

                    ui.label("ui frame");
                    ui.label(fmt_ms(timings.ui_frame.as_secs_f64()));
                    ui.end_row();

                    ui.label("drain_results");
                    ui.label(fmt_ms(timings.drain_results.as_secs_f64()));
                    ui.end_row();

                    ui.label("input_handlers");
                    ui.label(fmt_ms(timings.input_handlers.as_secs_f64()));
                    ui.end_row();

                    ui.label("show_pages");
                    ui.label(fmt_ms(timings.show_pages.as_secs_f64()));
                    ui.end_row();

                    ui.label("typst compile");
                    ui.label(fmt_ms(timings.typst_compile.as_secs_f64()));
                    ui.end_row();

                    ui.label("typst rasterize avg");
                    ui.label(fmt_ms(timings.typst_rasterize_avg.as_secs_f64()));
                    ui.end_row();
                });

            if !timings.render_pages.is_empty() {
                ui.separator();
                ui.label("Background renders:");
                egui::Grid::new("render_grid")
                    .num_columns(2)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        for (page, dur) in &timings.render_pages {
                            ui.label(format!("page {page}"));
                            ui.label(fmt_ms(dur.as_secs_f64()));
                            ui.end_row();
                        }
                    });
            }
        });
}

fn fmt_ms(secs: f64) -> String {
    format!("{:.2} ms", secs * 1000.0)
}
