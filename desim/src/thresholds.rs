use crate::sim;
use crate::sim::{Attribute, PlayerAtTick};
use serde::Deserialize;

static THRESHOLDS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/thresholds.json5"
));

#[derive(Debug, Deserialize)]
pub struct Thresholds {}

trait Vibable {
    fn vibed(self: &Self, vibes: f64) -> f64;
}

impl Vibable for f64 {
    fn vibed(self: &Self, vibes: f64) -> f64 {
        self * (1.0 + 0.2 * vibes)
    }
}

impl Thresholds {
    pub fn load() -> Result<Self, json5::Error> {
        json5::from_str(THRESHOLDS_JSON)
    }

    pub fn in_strike_zone(&self, game: &sim::GameAtTick) -> f64 {
        let pitcher_vibes = game.pitcher().vibes;

        let ruthlessness = game
            .pitcher()
            .attribute(Attribute::Ruthlessness)
            .multiplied();

        let musclitude = game
            .batter()
            .attribute(Attribute::Musclitude)
            .multiplied();

        let forwardness = 0.5; // Stadium attributes default to 0.5 when there is no stadium

        let threshold = 0.2 + 0.35 * (ruthlessness * (1.0 + 0.2 * pitcher_vibes)) + 0.2 * forwardness + 0.1 * musclitude;

        threshold.min(0.9)
    }

    #[allow(dead_code)] // TODO Remove once this is called for real
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

    pub fn fair_ball(&self, game: &sim::GameAtTick) -> f64 {
        let batter_vibes = game.batter().vibes;
        let forwardness = 0.5;
        let obtuseness = 0.5;

        let batter_musclitude = game
            .batter()
            .attribute(Attribute::Musclitude)
            .multiplied()
            .vibed(batter_vibes);

        let batter_thwackability = game
            .batter()
            .attribute(Attribute::Thwackability)
            .multiplied()
            .vibed(batter_vibes);

        let batter_divinity = game
            .batter()
            .attribute(Attribute::Divinity)
            .multiplied()
            .vibed(batter_vibes);

        let batter_sum = (batter_musclitude + batter_thwackability + batter_divinity) / 3.0;

        0.25 + 0.1 * forwardness - 0.1 * obtuseness + 0.1 * batter_sum
    }

    #[allow(dead_code)] // TODO Remove once this is called for real
    pub fn mild_pitch(&self) -> f64 {
        // Mysticism was always treated as 0.5 in s12
        let mysticism = 0.5;
        0.0005 + 0.004 * mysticism
    }

    pub fn out(&self, game: &sim::GameAtTick, fielder: &PlayerAtTick) -> f64 {
        let batter_vibes = game.batter().vibes;
        let pitcher_vibes = game.pitcher().vibes;
        let fielder_vibes = fielder.vibes;

        let batter_thwackability = game.batter()
            .attribute(Attribute::Thwackability)
            .multiplied()
            .vibed(batter_vibes);

        let pitcher_unthwackability = game.pitcher()
            .attribute(Attribute::Unthwackability)
            .multiplied()
            .vibed(pitcher_vibes);

        let fielder_omniscience = fielder
            .attribute(Attribute::Omniscience)
            .multiplied()
            .vibed(fielder_vibes);

        let stadium_grandiosity = 0.5;
        let stadium_ominousness = 0.5;
        let stadium_inconvenience = 0.5;
        let stadium_viscosity = 0.5;
        let stadium_forwardness = 0.5;

        0.315
            + 0.1 * batter_thwackability
            - 0.08 * pitcher_unthwackability
            - 0.07 * fielder_omniscience
            + 0.0145 * (stadium_grandiosity - 0.5)
            + 0.0085 * (stadium_ominousness - 0.5)
            - 0.011 * (stadium_inconvenience - 0.5)
            - 0.005 * (stadium_viscosity - 0.5)
            + 0.01 * (stadium_forwardness - 0.5)
    }

    pub fn fly(&self, game: &sim::GameAtTick) -> f64 {
        // No vibes, and inverse multiplied for some reason
        let batter_buoyancy = game.batter()
            .attribute(Attribute::Buoyancy)
            .inverse_multiplied();

        // Note for implementation: this needs to use the *pitcher*'s team.
        // Presumably it was supposed to use the pitcher's suppression as well
        // but TGB made a typo.
        // Also, no vibes
        let suppression = game.batter()
            .attribute(Attribute::Suppression)
            .multiplied();

        let stadium_ominousness = 0.5;

        let threshold = 0.18
            + 0.3 * batter_buoyancy
            - 0.16 * suppression
            - 0.1 * (stadium_ominousness - 0.5);
        threshold.max(0.01)
    }

    pub fn hr(&self, game: &sim::GameAtTick) -> f64 {
        let batter_vibes = game.batter().vibes;
        let pitcher_vibes = game.pitcher().vibes;

        let batter_divinity = game
            .batter()
            .attribute(Attribute::Divinity)
            .multiplied()
            .vibed(batter_vibes);

        let pitcher_overpowerment = game
            .pitcher()
            .attribute(Attribute::Overpowerment)
            .multiplied()
            .vibed(pitcher_vibes);

        let pitcher_suppression = game
            .pitcher()
            .attribute(Attribute::Suppression)
            .multiplied()
            .vibed(pitcher_vibes);

        let stadium_grandiosity = 0.5;
        let stadium_fortification = 0.5;
        let stadium_viscosity = 0.5;
        let stadium_ominousness = 0.5;
        let stadium_forwardness = 0.5;
        let ballpark_sum = 0.4 * (stadium_grandiosity - 0.5)
            + 0.2 * (stadium_fortification - 0.5)
            + 0.08 * (stadium_viscosity - 0.5)
            + 0.08 * (stadium_ominousness - 0.5)
            - 0.24 * (stadium_forwardness - 0.5);

        let opw_supp = (10.0 * pitcher_overpowerment + pitcher_suppression) / 11.0;
        0.12 + 0.16 * batter_divinity - 0.08 * opw_supp - 0.18 * ballpark_sum
    }

    pub fn double(&self, game: &sim::GameAtTick, fielder: &PlayerAtTick) -> f64 {
        let batter_vibes = game.batter().vibes;
        let pitcher_vibes = game.pitcher().vibes;
        let fielder_vibes = fielder.vibes;

        let batter_musclitude = game
            .batter()
            .attribute(Attribute::Musclitude)
            .multiplied()
            .vibed(batter_vibes);

        let pitcher_overpowerment = game
            .pitcher()
            .attribute(Attribute::Overpowerment)
            .multiplied()
            .vibed(pitcher_vibes);

        let fielder_chasiness = fielder
            .attribute(Attribute::Chasiness)
            .multiplied()
            .vibed(fielder_vibes);

        let stadium_forwardness = 0.5;
        let stadium_elongation = 0.5;
        let stadium_viscosity = 0.5;
        let stadium_ominousness = 0.5;
        let ballpark_sum = 0.027 * (stadium_forwardness - 0.5)
            - 0.015 * (stadium_elongation - 0.5)
            + 0.01 * (stadium_ominousness - 0.5)
            + 0.008 * (stadium_viscosity - 0.5);

        0.17 + 0.2 * batter_musclitude - 0.04 * pitcher_overpowerment - 0.1 * fielder_chasiness + ballpark_sum
    }

    pub fn triple(&self, game: &sim::GameAtTick, fielder: &PlayerAtTick) -> f64 {
        let batter_vibes = game.batter().vibes;
        let pitcher_vibes = game.pitcher().vibes;
        let fielder_vibes = fielder.vibes;

        let batter_ground_friction = game
            .batter()
            .attribute(Attribute::GroundFriction)
            .multiplied()
            .vibed(batter_vibes);

        let pitcher_overpowerment = game
            .pitcher()
            .attribute(Attribute::Overpowerment)
            .multiplied()
            .vibed(pitcher_vibes);

        let fielder_chasiness = fielder
            .attribute(Attribute::Chasiness)
            .multiplied()
            .vibed(fielder_vibes);

        let stadium_forwardness = 0.5;
        let stadium_grandiosity = 0.5;
        let stadium_obtuseness = 0.5;
        let stadium_viscosity = 0.5;
        let stadium_ominousness = 0.5;
        let ballpark_sum = (
            3.0 * (stadium_forwardness - 0.5)
            - 5.0 * (stadium_grandiosity - 0.5)
            + 5.0 * (stadium_obtuseness - 0.5)
            + 1.0 * (stadium_viscosity - 0.5)
            + 1.0 * (stadium_ominousness - 0.5)
        ) / 15.0;

        0.05 + 0.2 * batter_ground_friction - 0.04 * pitcher_overpowerment - 0.06 * fielder_chasiness + 0.1 * ballpark_sum
    }
}
