use crate::ir::{preset_category, preset_name, presets_for_category, PresetCategory};
use crate::params::{PresetId, TranslateParams};
use nih_plug::prelude::{Editor, Param, ParamSetter};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, RichText},
    widgets,
};
use std::sync::Arc;

pub fn create_editor(params: Arc<TranslateParams>) -> Option<Box<dyn Editor>> {
    let editor_state = params.editor_state.clone();

    create_egui_editor(
        editor_state,
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            draw_editor(egui_ctx, setter, &params);
        },
    )
}

fn draw_editor(egui_ctx: &egui::Context, setter: &ParamSetter, params: &TranslateParams) {
    let current_preset = params.preset.unmodulated_plain_value();

    egui::CentralPanel::default().show(egui_ctx, |ui| {
        ui.heading("TRANSLATE");
        ui.label("Milestone 3: preset bank, smoothing, and real response controls");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("Previous").clicked() {
                    set_preset(setter, params, current_preset.previous());
                }

                ui.vertical(|ui| {
                    ui.label(RichText::new(preset_name(current_preset)).strong());
                    ui.small(preset_category(current_preset).label());
                });

                if ui.button("Next").clicked() {
                    set_preset(setter, params, current_preset.next());
                }
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

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            draw_toggle(ui, "Mono", &params.mono, setter);
            draw_toggle(ui, "Bypass", &params.bypass, setter);
        });

        ui.add_space(12.0);
        ui.label(RichText::new("Preset Categories").strong());
        ui.add_space(4.0);

        for category in PresetCategory::ALL {
            ui.group(|ui| {
                ui.label(RichText::new(category.label()).strong());
                ui.horizontal_wrapped(|ui| {
                    for &preset in presets_for_category(category) {
                        let selected = current_preset == preset;
                        if ui.selectable_label(selected, preset_name(preset)).clicked() {
                            set_preset(setter, params, preset);
                        }
                    }
                });
            });
            ui.add_space(6.0);
        }

        ui.small(
            "Connected now: preset switching, decay, width, low EQ, high EQ, mix, output, mono.",
        );
    });
}

fn draw_slider<P: Param>(ui: &mut egui::Ui, label: &str, param: &P, setter: &ParamSetter) {
    ui.label(label);
    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(220.0));
}

fn set_preset(setter: &ParamSetter, params: &TranslateParams, preset: PresetId) {
    setter.begin_set_parameter(&params.preset);
    setter.set_parameter(&params.preset, preset);
    setter.end_set_parameter(&params.preset);
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
