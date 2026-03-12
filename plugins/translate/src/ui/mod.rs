use crate::params::{PresetMode, TranslateParams};
use nih_plug::prelude::{Editor, Enum as _, Param, ParamSetter};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, ComboBox, RichText},
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
    egui::CentralPanel::default().show(egui_ctx, |ui| {
        ui.heading("TRANSLATE");
        ui.label("Milestone 1 scaffold");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Preset").strong());
            draw_preset_selector(ui, setter, params);
        });

        ui.add_space(8.0);
        ui.columns(2, |columns| {
            draw_slider(&mut columns[0], "Decay", &params.decay, setter);
            draw_slider(&mut columns[1], "Mix", &params.mix, setter);
            draw_slider(&mut columns[0], "Width", &params.width, setter);
            draw_slider(&mut columns[1], "Low", &params.low, setter);
            draw_slider(&mut columns[0], "High", &params.high, setter);
            draw_slider(&mut columns[1], "Output", &params.output, setter);
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            draw_toggle(ui, "Mono", &params.mono, setter);
            draw_toggle(ui, "Bypass", &params.bypass, setter);
            draw_toggle(ui, "Quick Cycle", &params.quick_cycle, setter);
        });
    });
}

fn draw_slider<P: Param>(ui: &mut egui::Ui, label: &str, param: &P, setter: &ParamSetter) {
    ui.label(label);
    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(180.0));
}

fn draw_preset_selector(ui: &mut egui::Ui, setter: &ParamSetter, params: &TranslateParams) {
    let current = params.preset.unmodulated_plain_value();
    let current_name = PresetMode::variants()[current.to_index()];

    ComboBox::from_id_salt("preset-selector")
        .selected_text(current_name)
        .show_ui(ui, |ui| {
            for (index, name) in PresetMode::variants().iter().enumerate() {
                let variant = PresetMode::from_index(index);
                if ui.selectable_label(current == variant, *name).clicked() {
                    setter.begin_set_parameter(&params.preset);
                    setter.set_parameter(&params.preset, variant);
                    setter.end_set_parameter(&params.preset);
                }
            }
        });
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
