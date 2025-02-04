use serde::Deserialize;

static THRESHOLDS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/thresholds.json5"
));

#[derive(Debug, Deserialize)]
pub struct Thresholds {
    pub(crate) weather: f64,
}

impl Thresholds {
    pub fn load() -> Result<Self, json5::Error> {
        json5::from_str(THRESHOLDS_JSON)
    }

    pub fn in_strike_zone(&self) -> f64 {
        0.2 // TODO
    }

    pub fn made_contact(&self) -> f64 {
        0.2 // TODO
    }

    pub fn mild_pitch(&self) -> f64 {
        // Mysticism was always treated as 0.5 in s12
        let mysticism = 0.5;
        0.0005 + 0.004 * mysticism
    }
}
