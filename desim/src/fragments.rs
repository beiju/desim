use crate::rng::Rng;
use chrono::{DateTime, Utc};
use serde::Deserialize;

static FRAGMENTS_JSON5: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/fragments.json5"
));

#[derive(Debug, Clone, Deserialize)]
pub struct Fragment {
    pub label: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub rng: Rng,
    pub initial_step: i32,
}

pub type Fragments = Vec<Fragment>;

pub fn load_fragments() -> json5::Result<Fragments> {
    json5::from_str(FRAGMENTS_JSON5)
}
