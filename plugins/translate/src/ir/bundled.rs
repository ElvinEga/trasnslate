use hound::{SampleFormat, WavReader};
use std::io::Cursor;

pub const BUNDLED_IR_PATH: &str = "assets/irs/placeholder-room.wav";
const PLACEHOLDER_ROOM_IR: &[u8] = include_bytes!("../../../../assets/irs/placeholder-room.wav");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledIrId {
    PlaceholderRoom,
}

#[derive(Debug, Clone)]
pub struct BundledIr {
    pub id: BundledIrId,
    pub path: &'static str,
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
}

impl BundledIr {
    pub fn load(id: BundledIrId) -> Result<Self, String> {
        let (path, bytes) = match id {
            BundledIrId::PlaceholderRoom => (BUNDLED_IR_PATH, PLACEHOLDER_ROOM_IR),
        };

        let ir = decode_wav(bytes)?;
        Ok(Self { id, path, ir })
    }
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
