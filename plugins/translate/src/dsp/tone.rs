use std::f32::consts::PI;

const LOW_SHELF_FREQ_HZ: f32 = 180.0;
const HIGH_SHELF_FREQ_HZ: f32 = 4_200.0;

#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    fn set_coefficients(&mut self, coefficients: [f32; 5]) {
        self.b0 = coefficients[0];
        self.b1 = coefficients[1];
        self.b2 = coefficients[2];
        self.a1 = coefficients[3];
        self.a2 = coefficients[4];
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = input * self.b0 + self.z1;
        self.z1 = input * self.b1 - output * self.a1 + self.z2;
        self.z2 = input * self.b2 - output * self.a2;
        output
    }
}

#[derive(Debug, Default)]
pub struct StereoToneStack {
    low_left: Biquad,
    low_right: Biquad,
    high_left: Biquad,
    high_right: Biquad,
}

impl StereoToneStack {
    pub fn reset(&mut self) {
        self.low_left.reset();
        self.low_right.reset();
        self.high_left.reset();
        self.high_right.reset();
    }

    pub fn update(&mut self, sample_rate: f32, low_gain_db: f32, high_gain_db: f32) {
        let low = low_shelf(sample_rate, LOW_SHELF_FREQ_HZ, low_gain_db);
        let high = high_shelf(sample_rate, HIGH_SHELF_FREQ_HZ, high_gain_db);

        self.low_left.set_coefficients(low);
        self.low_right.set_coefficients(low);
        self.high_left.set_coefficients(high);
        self.high_right.set_coefficients(high);
    }

    pub fn process(&mut self, left: f32, right: f32) -> [f32; 2] {
        let left = self.high_left.process(self.low_left.process(left));
        let right = self.high_right.process(self.low_right.process(right));
        [left, right]
    }
}

fn low_shelf(sample_rate: f32, frequency: f32, gain_db: f32) -> [f32; 5] {
    shelf_coefficients(sample_rate, frequency, gain_db, true)
}

fn high_shelf(sample_rate: f32, frequency: f32, gain_db: f32) -> [f32; 5] {
    shelf_coefficients(sample_rate, frequency, gain_db, false)
}

fn shelf_coefficients(sample_rate: f32, frequency: f32, gain_db: f32, low: bool) -> [f32; 5] {
    if gain_db.abs() < 1.0e-4 {
        return [1.0, 0.0, 0.0, 0.0, 0.0];
    }

    let a = 10.0f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * frequency / sample_rate.max(1.0);
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * 2.0_f32.sqrt();
    let beta = 2.0 * a.sqrt() * alpha;

    let (b0, b1, b2, a0, a1, a2) = if low {
        (
            a * ((a + 1.0) - (a - 1.0) * cos_w0 + beta),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
            a * ((a + 1.0) - (a - 1.0) * cos_w0 - beta),
            (a + 1.0) + (a - 1.0) * cos_w0 + beta,
            -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
            (a + 1.0) + (a - 1.0) * cos_w0 - beta,
        )
    } else {
        (
            a * ((a + 1.0) + (a - 1.0) * cos_w0 + beta),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
            a * ((a + 1.0) + (a - 1.0) * cos_w0 - beta),
            (a + 1.0) - (a - 1.0) * cos_w0 + beta,
            2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
            (a + 1.0) - (a - 1.0) * cos_w0 - beta,
        )
    };

    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}
