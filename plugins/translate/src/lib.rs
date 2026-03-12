#![deny(unsafe_code)]

mod dsp;
mod ir;
mod params;
mod plugin;
mod ui;

pub use plugin::TranslatePlugin;

nih_plug::prelude::nih_export_clap!(TranslatePlugin);
nih_plug::prelude::nih_export_vst3!(TranslatePlugin);
