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
}
