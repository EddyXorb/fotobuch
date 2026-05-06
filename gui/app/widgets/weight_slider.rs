use crate::state::{InteractionState, WeightSlider};
use crate::task::BackgroundTask;

pub fn show(
    ctx: &egui::Context,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    let (page, slots, pos, value) = match &interaction.weight_slider {
        WeightSlider::Open {
            page,
            slots,
            screen_pos,
            value,
        } => (*page, slots.clone(), *screen_pos, *value),
        WeightSlider::Closed => return,
    };

    let mut current_value = value;

    let resp = egui::Area::new(egui::Id::new("weight_slider"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let slider = egui::Slider::new(&mut current_value, 0.1..=10.0)
                        .step_by(0.1)
                        .vertical();
                    let resp = ui.add(slider);

                    if resp.drag_stopped() {
                        cmds.push(BackgroundTask::SetWeight {
                            page,
                            slots: slots.clone(),
                            weight: current_value,
                        });
                        interaction.weight_slider = WeightSlider::Closed;
                    }
                });
            });
        });

    // Update live value.
    if let WeightSlider::Open { value, .. } = &mut interaction.weight_slider {
        *value = current_value;
    }

    // Close on Escape or click outside.
    let esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    let lmb_outside = ctx.input(|i| i.pointer.primary_pressed())
        && !resp
            .response
            .rect
            .contains(ctx.pointer_interact_pos().unwrap_or_default());

    if esc || lmb_outside {
        interaction.weight_slider = WeightSlider::Closed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_slider_closed_by_default() {
        let slider = WeightSlider::default();
        assert!(!slider.is_open());
    }

    #[test]
    fn weight_slider_open_is_open() {
        let slider = WeightSlider::Open {
            page: 0,
            slots: vec![0, 1],
            screen_pos: egui::pos2(0.0, 0.0),
            value: 1.5,
        };
        assert!(slider.is_open());
    }
}
