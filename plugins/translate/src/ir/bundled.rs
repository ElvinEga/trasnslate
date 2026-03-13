use crate::params::PresetId;
use hound::{SampleFormat, WavReader};
use nih_plug::prelude::Enum;
use std::io::Cursor;

const CAR_HATCHBACK_IR: &[u8] = include_bytes!("../../../../assets/irs/car-hatchback.wav");
const PHONE_SPEAKER_IR: &[u8] = include_bytes!("../../../../assets/irs/phone-speaker.wav");
const TABLET_LAPTOP_IR: &[u8] = include_bytes!("../../../../assets/irs/tablet-laptop.wav");
const CLUB_BOOTH_IR: &[u8] = include_bytes!("../../../../assets/irs/club-booth.wav");
const CONCERT_VENUE_IR: &[u8] = include_bytes!("../../../../assets/irs/concert-venue.wav");
const MONO_RADIO_IR: &[u8] = include_bytes!("../../../../assets/irs/mono-radio.wav");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetCategory {
    Cars,
    Phones,
    TabletsLaptops,
    Clubs,
    ConcertVenues,
    MonoDevices,
}

impl PresetCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cars => "Cars",
            Self::Phones => "Phones",
            Self::TabletsLaptops => "Tablets / Laptops",
            Self::Clubs => "Clubs",
            Self::ConcertVenues => "Concert / Venues",
            Self::MonoDevices => "Mono devices",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BundledIr {
    pub id: PresetId,
    pub ir: StereoIr,
}

#[derive(Debug, Clone)]
pub struct PreparedPreset {
    pub id: PresetId,
    pub ir: StereoIr,
}

#[derive(Debug, Clone)]
pub struct StereoIr {
    pub sample_rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl StereoIr {
    pub fn len(&self) -> usize {
        self.left.len().min(self.right.len())
    }

    pub fn resampled(&self, target_sample_rate: u32) -> Self {
        if self.sample_rate == target_sample_rate || self.len() <= 1 {
            return self.clone();
        }

        let ratio = target_sample_rate as f64 / self.sample_rate as f64;
        let target_len = ((self.len() as f64 * ratio).round() as usize).max(1);

        Self {
            sample_rate: target_sample_rate,
            left: resample_channel(&self.left, target_len),
            right: resample_channel(&self.right, target_len),
        }
    }
}

pub fn preset_name(id: PresetId) -> &'static str {
    descriptor(id).name
}

pub fn preset_category(id: PresetId) -> PresetCategory {
    descriptor(id).category
}

pub fn preset_filename(id: PresetId) -> &'static str {
    descriptor(id).file_name
}

impl BundledIr {
    pub fn load_all() -> Result<Vec<Self>, String> {
        let mut presets = Vec::with_capacity(PresetId::variants().len());
        for id in [
            PresetId::CarHatchback,
            PresetId::PhoneSpeaker,
            PresetId::TabletLaptop,
            PresetId::ClubBooth,
            PresetId::ConcertVenue,
            PresetId::MonoRadio,
        ] {
            presets.push(load_one(id)?);
        }

        Ok(presets)
    }
}

struct BundledDescriptor {
    id: PresetId,
    category: PresetCategory,
    name: &'static str,
    file_name: &'static str,
    bytes: &'static [u8],
}

fn descriptor(id: PresetId) -> BundledDescriptor {
    match id {
        PresetId::CarHatchback => BundledDescriptor {
            id,
            category: PresetCategory::Cars,
            name: "Hatchback",
            file_name: "car-hatchback.wav",
            bytes: CAR_HATCHBACK_IR,
        },
        PresetId::PhoneSpeaker => BundledDescriptor {
            id,
            category: PresetCategory::Phones,
            name: "Phone Speaker",
            file_name: "phone-speaker.wav",
            bytes: PHONE_SPEAKER_IR,
        },
        PresetId::TabletLaptop => BundledDescriptor {
            id,
            category: PresetCategory::TabletsLaptops,
            name: "Tablet / Laptop",
            file_name: "tablet-laptop.wav",
            bytes: TABLET_LAPTOP_IR,
        },
        PresetId::ClubBooth => BundledDescriptor {
            id,
            category: PresetCategory::Clubs,
            name: "Club Booth",
            file_name: "club-booth.wav",
            bytes: CLUB_BOOTH_IR,
        },
        PresetId::ConcertVenue => BundledDescriptor {
            id,
            category: PresetCategory::ConcertVenues,
            name: "Concert Venue",
            file_name: "concert-venue.wav",
            bytes: CONCERT_VENUE_IR,
        },
        PresetId::MonoRadio => BundledDescriptor {
            id,
            category: PresetCategory::MonoDevices,
            name: "Mono Radio",
            file_name: "mono-radio.wav",
            bytes: MONO_RADIO_IR,
        },
    }
}

fn load_one(id: PresetId) -> Result<BundledIr, String> {
    let descriptor = descriptor(id);
    let ir = decode_wav(descriptor.bytes)?;
    Ok(BundledIr {
        id: descriptor.id,
        ir,
    })
}

fn decode_wav(bytes: &[u8]) -> Result<StereoIr, String> {
    let mut reader = WavReader::new(Cursor::new(bytes))
        .map_err(|error| format!("failed to open bundled IR WAV: {error}"))?;
    let spec = reader.spec();

    if spec.channels == 0 || spec.channels > 2 {
        return Err(format!(
            "unsupported channel count for bundled IR: {}",
            spec.channels
        ));
    }

    let interleaved = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to decode float WAV IR: {error}"))?,
        (SampleFormat::Int, bits) if bits <= 32 => {
            let max_amplitude = ((1_i64 << (bits - 1)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / max_amplitude)
                        .map_err(|error| format!("failed to decode PCM WAV IR: {error}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        _ => {
            return Err(format!(
                "unsupported WAV IR format: {:?} {} bits",
                spec.sample_format, spec.bits_per_sample
            ))
        }
    };

    let mut left = Vec::with_capacity(interleaved.len() / spec.channels as usize);
    let mut right = Vec::with_capacity(interleaved.len() / spec.channels as usize);

    if spec.channels == 1 {
        left.extend(interleaved.iter().copied());
        right.extend(interleaved.iter().copied());
    } else {
        for frame in interleaved.chunks_exact(2) {
            left.push(frame[0]);
            right.push(frame[1]);
        }
    }

    if left.is_empty() {
        return Err("bundled IR decoded to zero samples".to_string());
    }

    Ok(StereoIr {
        sample_rate: spec.sample_rate,
        left,
        right,
    })
}

fn resample_channel(channel: &[f32], target_len: usize) -> Vec<f32> {
    if channel.len() <= 1 || target_len <= 1 {
        return vec![channel[0]];
    }

    let scale = (channel.len() - 1) as f64 / (target_len - 1) as f64;
    let mut out = Vec::with_capacity(target_len);
    for index in 0..target_len {
        let source_pos = index as f64 * scale;
        let base = source_pos.floor() as usize;
        let frac = (source_pos - base as f64) as f32;
        let next = (base + 1).min(channel.len() - 1);
        let sample = channel[base] * (1.0 - frac) + channel[next] * frac;
        out.push(sample);
    }

    out
}
