use std::f32::consts::PI;

pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn new(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

pub struct Eq {
    bands: Vec<Biquad>,
    pub enabled: bool,
}

impl Eq {
    pub fn new(bands: Vec<Biquad>, enabled: bool) -> Self {
        Self { bands, enabled }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }

        let mut x = sample;
        for band in &mut self.bands {
            x = band.process(x);
        }
        x
    }

    pub fn update_bands(&mut self, bands: Vec<Biquad>) {
        self.bands = bands;
        for band in &mut self.bands {
            band.reset();
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn from_config(eq_bands: Vec<[f32; 4]>, enabled: bool, sample_rate: f32) -> Self {
        let bands: Vec<Biquad> = eq_bands
            .into_iter()
            .map(|band| {
                let f0 = band[0];
                let q = band[1];
                let gain_db = band[2];
                let band_type = band[3] as u8;
                let (b0, b1, b2, a1, a2) =
                    biquad_coefficients(f0, q, gain_db, band_type, sample_rate);
                Biquad::new(b0, b1, b2, a1, a2)
            })
            .collect();

        Self { bands, enabled }
    }
}

fn biquad_coefficients(
    f0: f32,
    q: f32,
    gain_db: f32,
    band_type: u8,
    sample_rate: f32,
) -> (f32, f32, f32, f32, f32) {
    let a = 10f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * f0 / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    match band_type {
        0 => {
            // low shelf
            let sqrt_a = a.sqrt();
            let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
            let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
            let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
            let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
            (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
        }
        1 => {
            // peaking EQ
            let b0 = 1.0 + alpha * a;
            let b1 = -2.0 * cos_w0;
            let b2 = 1.0 - alpha * a;
            let a0 = 1.0 + alpha / a;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha / a;
            (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
        }
        2 => {
            // high shelf
            let sqrt_a = a.sqrt();
            let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
            let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
            let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
            let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
            (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
        }
        _ => (1.0, 0.0, 0.0, 0.0, 0.0),
    }
}
