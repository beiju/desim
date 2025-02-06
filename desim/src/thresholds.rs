use crate::sim;
use crate::sim::Attribute;
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

    pub fn swing(&self, in_zone: bool, game: &sim::GameAtTick) -> f64 {
        if in_zone {
            self.swing_on_pitch_in_zone(game)
        } else {
            self.swing_on_pitch_not_in_zone(game)
        }
    }

    pub fn swing_on_pitch_in_zone(&self, game: &sim::GameAtTick) -> f64 {
        // The entire body of this function comes from get_swing_strike_threshold in resim's
        // formulas.py
        let batter = game.batter();
        let divinity = batter.attribute(Attribute::Divinity).multiplied();
        let muscularity = batter.attribute(Attribute::Musclitude).multiplied();
        let inverse_patheticism = batter
            .attribute(Attribute::Patheticism)
            .inverse_multiplied();
        let thwackability = batter.attribute(Attribute::Thwackability).multiplied();
        let combined_batting = (divinity + muscularity + inverse_patheticism + thwackability) / 4.;

        let ruthlessness = game
            .pitcher()
            .attribute(Attribute::Ruthlessness)
            .multiplied();

        let viscosity = 0.5; // Ballpark attributes are mostly assumed 0.5 before parks existed

        0.7 + 0.35 * combined_batting - 0.4 * ruthlessness + 0.2 * (viscosity - 0.5)
    }

    pub fn swing_on_pitch_not_in_zone(&self, game: &sim::GameAtTick) -> f64 {
        // The entire body of this function comes from get_swing_strike_threshold in resim's
        // formulas.py
        let batter = game.batter();
        let moxie = batter.attribute(Attribute::Moxie).multiplied();
        let patheticism = batter
            .attribute(Attribute::Patheticism)
            .inverse_multiplied();

        let ruthlessness = game
            .pitcher()
            .attribute(Attribute::Ruthlessness)
            .multiplied();

        let viscosity = 0.5; // Ballpark attributes are mostly assumed 0.5 before parks existed

        let combined = (12. * ruthlessness - 5. * moxie + 5. * patheticism + 4. * viscosity) / 20.;
        combined.powf(1.5).min(0.95).max(0.1)
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
