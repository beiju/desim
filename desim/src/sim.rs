use blaseball_api::{chronicler, ChroniclerGameUpdate};
use chrono::{DateTime, Utc};
use enum_map::{enum_map, Enum, EnumMap};
use rocket::futures::{stream, StreamExt};
use uuid::Uuid;

pub struct GameTeam {
    lineup: Vec<Player>,
    pitcher: Player,
}

impl GameTeam {
    pub async fn fetch(team_id: Uuid, at_time: DateTime<Utc>) -> Self {
        let team = chronicler::team(team_id, at_time)
            .await
            .expect("TODO: Handle failure to fetch team");
        let lineup = stream::iter(team.data.lineup)
            .then(|player_id| Player::fetch(player_id, at_time))
            .collect()
            .await;

        // Not sure if solidifying the pitcher this early will come back to bite me
        // TODO This +1 was the quickest way to get the right pitcher loaded for the one game I
        //   started with, but I don't know why it's here
        let rotation_index = (team.data.rotation_slot + 1) as usize % team.data.rotation.len();
        let pitcher_uuid = team.data.rotation[rotation_index];
        let pitcher = Player::fetch(pitcher_uuid, at_time).await;

        Self { lineup, pitcher }
    }
}

pub struct Game {
    away: GameTeam,
    home: GameTeam,
}

impl Game {
    pub async fn from_first_game_update(first_update: &ChroniclerGameUpdate) -> Self {
        let away = GameTeam::fetch(first_update.data.away_team, first_update.timestamp).await;
        let home = GameTeam::fetch(first_update.data.home_team, first_update.timestamp).await;
        Game { away, home }
    }

    pub fn at_tick(&self, game_update: &ChroniclerGameUpdate) -> GameAtTick {
        let half = if game_update.data.top_of_inning {
            HalfInning::Top
        } else {
            HalfInning::Bottom
        };
        GameAtTick {
            game: self,
            half,
            batter_count: match half {
                HalfInning::Top => game_update.data.away_team_batter_count,
                HalfInning::Bottom => game_update.data.home_team_batter_count,
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum HalfInning {
    Top,
    Bottom,
}

// `Game` is data that's true for the whole game, while GameState may change per tick
pub struct GameAtTick<'a> {
    pub game: &'a Game,
    pub half: HalfInning,
    // I found it more convenient to use the batter count, which I can convert into an index into
    // the lineup, rather than the batter ID which I would have to search the lineup for
    pub batter_count: i64,
}

impl<'a> GameAtTick<'a> {
    pub fn pitching_team(&self) -> &'a GameTeam {
        // This has to be the wrong way round, right? And yet it's giving me the right pitcher.
        // what
        match self.half {
            HalfInning::Top => &self.game.away,
            HalfInning::Bottom => &self.game.home,
        }
    }

    pub fn pitcher(&self) -> PlayerAtTick {
        PlayerAtTick {
            player: &self.pitching_team().pitcher,
        }
    }

    pub fn batting_team(&self) -> &'a GameTeam {
        match self.half {
            HalfInning::Top => &self.game.away,
            HalfInning::Bottom => &self.game.home,
        }
    }

    pub fn batter(&self) -> PlayerAtTick {
        let batting_lineup = &self.batting_team().lineup;
        if self.batter_count < 0 {
            panic!("I don't know what this means")
        }
        let index = self.batter_count as usize % batting_lineup.len();
        PlayerAtTick {
            player: &batting_lineup[index],
        }
    }
}

// TODO Attributes have numeric ids, those should used as the repr
#[derive(Debug, Copy, Clone, Enum)]
pub enum Attribute {
    // Batting
    Buoyancy,
    Divinity,
    Martyrdom,
    Moxie,
    Musclitude,
    Patheticism,
    Thwackability,
    Tragicness,
    // Pitching
    Ruthlessness,
    Overpowerment,
    Unthwackability,
    Shakespearianism,
    Suppression,
    Coldness,
    // Baserunning
    BaseThirst,
    Laserlikeness,
    Continuation,
    GroundFriction,
    Indulgence,
    // Defense
    Anticapitalism,
    Chasiness,
    Omniscience,
    Tenaciousness,
    Watchfulness,
}

pub struct Player {
    pub id: Uuid,
    attributes: EnumMap<Attribute, f64>,
}

impl Player {
    pub async fn fetch(player_id: Uuid, at_time: DateTime<Utc>) -> Self {
        let player = chronicler::player(player_id, at_time)
            .await
            .expect("TODO: Handle failure to fetch player");
        assert_eq!(player_id, player.entity_id);
        assert_eq!(player_id, player.data.id);

        Self {
            id: player_id,
            attributes: enum_map! {
                Attribute::Buoyancy => player.data.buoyancy,
                Attribute::Divinity => player.data.divinity,
                Attribute::Martyrdom => player.data.martyrdom,
                Attribute::Moxie => player.data.moxie,
                Attribute::Musclitude => player.data.musclitude,
                Attribute::Patheticism => player.data.patheticism,
                Attribute::Thwackability => player.data.thwackability,
                Attribute::Tragicness => player.data.tragicness,
                Attribute::Ruthlessness => player.data.ruthlessness,
                Attribute::Overpowerment => player.data.overpowerment,
                Attribute::Unthwackability => player.data.unthwackability,
                Attribute::Shakespearianism => player.data.shakespearianism,
                Attribute::Suppression => player.data.suppression,
                Attribute::Coldness => player.data.coldness,
                Attribute::BaseThirst => player.data.base_thirst,
                Attribute::Laserlikeness => player.data.laserlikeness,
                Attribute::Continuation => player.data.continuation,
                Attribute::GroundFriction => player.data.ground_friction,
                Attribute::Indulgence => player.data.indulgence,
                Attribute::Anticapitalism => player.data.anticapitalism,
                Attribute::Chasiness => player.data.chasiness,
                Attribute::Omniscience => player.data.omniscience,
                Attribute::Tenaciousness => player.data.tenaciousness,
                Attribute::Watchfulness => player.data.watchfulness,
            },
        }
    }
}

pub struct PlayerAtTick<'a> {
    pub player: &'a Player,
}

impl<'a> PlayerAtTick<'a> {
    pub fn attribute(&self, attr: Attribute) -> AttributeValue {
        AttributeValue::new(attr, self.player.attributes[attr])
    }
}

pub struct AttributeValue {
    pub attribute: Attribute,
    pub base_value: f64,
}

impl AttributeValue {
    pub fn new(attribute: Attribute, base_value: f64) -> Self {
        Self {
            attribute,
            base_value,
        }
    }

    fn compute_multiplier(&self) -> f64 {
        // TODO: All the multiplier code. It's a lot.
        1.
    }

    pub fn multiplied(&self) -> f64 {
        // Note for when items are implemented: item contribution to attributes
        // does not get multiplied
        self.base_value * self.compute_multiplier()
    }

    pub fn inverse_multiplied(&self) -> f64 {
        // Note for when items are implemented: item contribution to attributes
        // does not get multiplied
        self.base_value * 1. / self.compute_multiplier()
    }
}
