use crate::ir::{preset_category, preset_filename, preset_name};
use crate::params::{QuickCycleMode, TranslateParams};
use crate::quick_cycle::{QuickCycleAction, QuickCycleShared};
use crate::workflow::{CompareSlotId, ParameterSnapshot, WorkflowShared};
use nih_plug::prelude::{
    BoolParam, Editor, Enum, EnumParam, FloatParam, IntParam, Param, ParamSetter,
};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, RichText},
    widgets,
};
use std::sync::Arc;

pub fn create_editor(
    params: Arc<TranslateParams>,
    quick_cycle: Arc<QuickCycleShared>,
    workflow: Arc<WorkflowShared>,
) -> Option<Box<dyn Editor>> {
    let editor_state = params.editor_state.clone();

    create_egui_editor(
        editor_state,
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            draw_editor(
                egui_ctx,
                setter,
                &params,
                quick_cycle.as_ref(),
                workflow.as_ref(),
            );
        },
    )
}

fn draw_editor(
    egui_ctx: &egui::Context,
    setter: &ParamSetter,
    params: &TranslateParams,
    quick_cycle: &QuickCycleShared,
    workflow: &WorkflowShared,
) {
    let cycle_snapshot = quick_cycle.snapshot();
    let workflow_snapshot = workflow.snapshot();
    let current_display = workflow_snapshot
        .active_preset
        .or_else(|| quick_cycle.current_preset())
        .unwrap_or_else(|| params.preset.unmodulated_plain_value());
    let next_display = quick_cycle.next_preset();
    let mode = params.quick_cycle_mode.unmodulated_plain_value();

    egui::CentralPanel::default().show(egui_ctx, |ui| {
        ui.heading("TRANSLATE");
        ui.label("Monitoring and workflow");
        ui.add_space(10.0);

        ui.columns(2, |columns| {
            columns[0].group(|ui| {
                ui.label(RichText::new("Status").strong());
                ui.label(format!("Preset: {}", preset_name(current_display)));
                ui.small(preset_category(current_display).label());
                ui.label(format!("IR File: {}", preset_filename(current_display)));
                ui.label(format!(
                    "IR Length: {} samples",
                    workflow_snapshot.ir_samples
                ));
                ui.label(format!(
                    "Sample Rate: {:.0} Hz",
                    workflow_snapshot.sample_rate
                ));
                ui.label(format!(
                    "Latency: {} samples",
                    workflow_snapshot.latency_samples
                ));
                ui.label(format!(
                    "Loudness Comp: {:+.1} dB",
                    gain_to_db(workflow_snapshot.loudness_compensation_gain)
                ));
                ui.label(format!(
                    "Limiter GR: {:.1} dB",
                    workflow_snapshot.limiter_gain_reduction_db
                ));
                ui.label(if workflow_snapshot.bypass_active {
                    "Bypass: Active"
                } else {
                    "Bypass: Off"
                });
            });

            columns[1].group(|ui| {
                ui.label(RichText::new("Meters").strong());
                draw_meter(ui, "Input", workflow_snapshot.input_peak);
                draw_meter(ui, "Output", workflow_snapshot.output_peak);
            });
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(RichText::new("A / B Compare").strong());
            ui.horizontal(|ui| {
                if ui.button("Store A").clicked() {
                    workflow.store_a(ParameterSnapshot::from_params(params));
                }
                let use_a = ui.add_enabled(
                    workflow_snapshot.has_a_snapshot,
                    egui::Button::new(ab_button_label(
                        "Use A",
                        workflow_snapshot.active_compare_slot == CompareSlotId::A,
                    )),
                );
                if use_a.clicked() {
                    if let Some(snapshot) = workflow.recall_a() {
                        apply_snapshot(setter, params, snapshot);
                        workflow.set_active_compare_slot(CompareSlotId::A);
                    }
                }

                if ui.button("Store B").clicked() {
                    workflow.store_b(ParameterSnapshot::from_params(params));
                }
                let use_b = ui.add_enabled(
                    workflow_snapshot.has_b_snapshot,
                    egui::Button::new(ab_button_label(
                        "Use B",
                        workflow_snapshot.active_compare_slot == CompareSlotId::B,
                    )),
                );
                if use_b.clicked() {
                    if let Some(snapshot) = workflow.recall_b() {
                        apply_snapshot(setter, params, snapshot);
                        workflow.set_active_compare_slot(CompareSlotId::B);
                    }
                }
            });
        });

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
            draw_slider(&mut columns[1], "Output Trim", &params.output, setter);
            draw_slider(&mut columns[0], "Low EQ", &params.low, setter);
            draw_slider(&mut columns[1], "High EQ", &params.high, setter);
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(RichText::new("Workflow Settings").strong());
            draw_enum_selector(ui, setter, "Preset", &params.preset);
            draw_enum_selector(ui, setter, "Quick Cycle Mode", &params.quick_cycle_mode);
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
                "Loudness Lock",
                &params.quick_cycle_loudness_lock,
                setter,
            );
            draw_toggle(
                ui,
                "Return to Reference on stop",
                &params.quick_cycle_return_to_reference,
                setter,
            );
            draw_toggle(ui, "Safety Limiter", &params.safety_limiter, setter);
        });

        ui.add_space(10.0);
        ui.label(RichText::new("Cycle List").strong());
        for slot in 0..cycle_snapshot.slots.len() {
            let cycle_slot = cycle_snapshot.slots[slot];
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

fn draw_meter(ui: &mut egui::Ui, label: &str, linear_peak: f32) {
    let db = linear_to_db(linear_peak);
    let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    ui.label(format!("{label}: {db:.1} dBFS"));
    ui.add(egui::ProgressBar::new(normalized).show_percentage());
}

fn ab_button_label(label: &str, active: bool) -> String {
    if active {
        format!("{label} *")
    } else {
        label.to_string()
    }
}

fn apply_snapshot(setter: &ParamSetter, params: &TranslateParams, snapshot: ParameterSnapshot) {
    set_enum_param(setter, &params.preset, snapshot.preset);
    set_float_param(setter, &params.decay, snapshot.decay);
    set_float_param(setter, &params.mix, snapshot.mix);
    set_float_param(setter, &params.width, snapshot.width);
    set_float_param(setter, &params.low, snapshot.low);
    set_float_param(setter, &params.high, snapshot.high);
    set_float_param(setter, &params.output, snapshot.output);
    set_bool_param(setter, &params.mono, snapshot.mono);
    set_bool_param(setter, &params.bypass, snapshot.bypass);
    set_enum_param(setter, &params.quick_cycle_mode, snapshot.quick_cycle_mode);
    set_int_param(
        setter,
        &params.quick_cycle_switch_time_ms,
        snapshot.quick_cycle_switch_time_ms,
    );
    set_int_param(
        setter,
        &params.quick_cycle_crossfade_ms,
        snapshot.quick_cycle_crossfade_ms,
    );
    set_bool_param(
        setter,
        &params.quick_cycle_loudness_lock,
        snapshot.loudness_lock,
    );
    set_bool_param(
        setter,
        &params.quick_cycle_return_to_reference,
        snapshot.quick_cycle_return_to_reference,
    );
    set_bool_param(setter, &params.safety_limiter, snapshot.safety_limiter);
}

fn draw_slider<P: Param>(ui: &mut egui::Ui, label: &str, param: &P, setter: &ParamSetter) {
    ui.label(label);
    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(240.0));
}

fn draw_toggle(ui: &mut egui::Ui, label: &str, param: &BoolParam, setter: &ParamSetter) {
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

fn set_float_param(setter: &ParamSetter, param: &FloatParam, value: f32) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}

fn set_int_param(setter: &ParamSetter, param: &IntParam, value: i32) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}

fn set_bool_param(setter: &ParamSetter, param: &BoolParam, value: bool) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}

fn set_enum_param<E: Enum + PartialEq>(setter: &ParamSetter, param: &EnumParam<E>, value: E) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}

fn linear_to_db(value: f32) -> f32 {
    20.0 * value.max(1.0e-6).log10()
}

fn gain_to_db(value: f32) -> f32 {
    if (value - 1.0).abs() < 1.0e-6 {
        0.0
    } else {
        linear_to_db(value)
    }
}
