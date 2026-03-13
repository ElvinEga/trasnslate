use crate::ir::{preset_category, preset_name};
use crate::params::{QuickCycleMode, TranslateParams};
use crate::quick_cycle::{QuickCycleAction, QuickCycleShared};
use nih_plug::prelude::{Editor, Param, ParamSetter};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, RichText},
    widgets,
};
use std::sync::Arc;

pub fn create_editor(
    params: Arc<TranslateParams>,
    quick_cycle: Arc<QuickCycleShared>,
) -> Option<Box<dyn Editor>> {
    let editor_state = params.editor_state.clone();

    create_egui_editor(
        editor_state,
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            draw_editor(egui_ctx, setter, &params, quick_cycle.as_ref());
        },
    )
}

fn draw_editor(
    egui_ctx: &egui::Context,
    setter: &ParamSetter,
    params: &TranslateParams,
    quick_cycle: &QuickCycleShared,
) {
    let snapshot = quick_cycle.snapshot();
    let current_display = quick_cycle
        .current_preset()
        .unwrap_or_else(|| params.preset.unmodulated_plain_value());
    let next_display = quick_cycle.next_preset();
    let mode = params.quick_cycle_mode.unmodulated_plain_value();

    egui::CentralPanel::default().show(egui_ctx, |ui| {
        ui.heading("TRANSLATE");
        ui.label("Quick Cycle");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Cycle Transport").strong());
            ui.horizontal(|ui| {
                if ui.button("Previous").clicked() {
                    quick_cycle.request_action(QuickCycleAction::Previous);
                }

                let start_label = if mode == QuickCycleMode::Manual {
                    "Cycle"
                } else if quick_cycle.is_running() {
                    "Running"
                } else {
                    "Start"
                };
                if ui.button(start_label).clicked() {
                    quick_cycle.request_action(QuickCycleAction::StartOrCycle);
                }

                let stop_label = if mode == QuickCycleMode::Timed {
                    "Pause"
                } else {
                    "Stop"
                };
                if ui.button(stop_label).clicked() {
                    quick_cycle.request_action(QuickCycleAction::PauseOrStop);
                }

                if ui.button("Next").clicked() {
                    quick_cycle.request_action(QuickCycleAction::Next);
                }

                if ui.button("Return to Reference").clicked() {
                    quick_cycle.request_action(QuickCycleAction::ReturnToReference);
                }
            });
        });

        ui.add_space(8.0);
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Current").strong());
                    ui.label(preset_name(current_display));
                    ui.small(preset_category(current_display).label());
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.label(RichText::new("Next").strong());
                    if let Some(next_display) = next_display {
                        ui.label(preset_name(next_display));
                        ui.small(preset_category(next_display).label());
                    } else {
                        ui.label("Reference");
                        ui.small("No queued cycle step");
                    }
                });
            });
        });

        ui.add_space(10.0);
        ui.columns(2, |columns| {
            draw_slider(&mut columns[0], "Decay", &params.decay, setter);
            draw_slider(&mut columns[1], "Mix", &params.mix, setter);
            draw_slider(&mut columns[0], "Width", &params.width, setter);
            draw_slider(&mut columns[1], "Output", &params.output, setter);
            draw_slider(&mut columns[0], "Low EQ", &params.low, setter);
            draw_slider(&mut columns[1], "High EQ", &params.high, setter);
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(RichText::new("Quick Cycle Settings").strong());
            draw_enum_selector(ui, setter, "Mode", &params.quick_cycle_mode);
            draw_slider(
                ui,
                "Switch Time",
                &params.quick_cycle_switch_time_ms,
                setter,
            );
            draw_slider(
                ui,
                "Crossfade Time",
                &params.quick_cycle_crossfade_ms,
                setter,
            );
            draw_toggle(
                ui,
                "Loudness Lock (placeholder)",
                &params.quick_cycle_loudness_lock,
                setter,
            );
            draw_toggle(
                ui,
                "Return to Reference on stop",
                &params.quick_cycle_return_to_reference,
                setter,
            );
        });

        ui.add_space(10.0);
        ui.label(RichText::new("Cycle List").strong());
        for slot in 0..snapshot.slots.len() {
            let cycle_slot = snapshot.slots[slot];
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let mut enabled = quick_cycle.is_slot_enabled(slot);
                    if ui.checkbox(&mut enabled, "").changed() {
                        quick_cycle.set_slot_enabled(slot, enabled);
                    }

                    ui.label(format!("{}. {}", slot + 1, preset_name(cycle_slot.preset)));

                    if ui.small_button("Up").clicked() {
                        quick_cycle.move_up(slot);
                    }
                    if ui.small_button("Down").clicked() {
                        quick_cycle.move_down(slot);
                    }
                });
            });
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            draw_toggle(ui, "Mono", &params.mono, setter);
            draw_toggle(ui, "Bypass", &params.bypass, setter);
        });
    });
}

fn draw_slider<P: Param>(ui: &mut egui::Ui, label: &str, param: &P, setter: &ParamSetter) {
    ui.label(label);
    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(240.0));
}

fn draw_toggle(
    ui: &mut egui::Ui,
    label: &str,
    param: &nih_plug::prelude::BoolParam,
    setter: &ParamSetter,
) {
    let mut value = param.unmodulated_plain_value();
    if ui.checkbox(&mut value, label).changed() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

fn draw_enum_selector<T: Param>(ui: &mut egui::Ui, setter: &ParamSetter, label: &str, param: &T) {
    ui.label(label);
    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(240.0));
}
