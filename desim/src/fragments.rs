use crate::rng::Rng;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

static FRAGMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/fragments.json"
));

pub type Fragments = HashMap<DateTime<Utc>, Rng>;

pub fn load_fragments() -> serde_json::Result<Fragments> {
    serde_json::from_str(FRAGMENTS_JSON)
}
