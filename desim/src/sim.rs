use blaseball_api::ChroniclerGameUpdate;

pub struct Game {}

impl Game {
    pub fn from_first_event() -> Self {
        Game {}
    }

    pub fn at_tick(&self, game_update: &ChroniclerGameUpdate) -> GameAtTick {
        GameAtTick {
            game: self,
            half: if game_update.data.top_of_inning {
                HalfInning::Top
            } else {
                HalfInning::Bottom
            },
        }
    }
}

pub enum HalfInning {
    Top,
    Bottom,
}

// `Game` is data that's true for the whole game, while GameState may change per tick
pub struct GameAtTick<'a> {
    game: &'a Game,
    half: HalfInning,
}

impl<'a> GameAtTick<'a> {
    pub fn pitcher(&self) -> &PlayerAtTick {
        todo!()
    }

    pub fn batter(&self) -> &PlayerAtTick {
        todo!()
    }
}

pub struct Player {}

pub struct PlayerAtTick<'a> {
    player: &'a Player,
}

impl<'a> PlayerAtTick<'a> {
    pub fn ruthlessness(&self) -> f64 {
        todo!()
    }

    pub fn thwackability(&self) -> f64 {
        todo!()
    }

    pub fn patheticism(&self) -> f64 {
        todo!()
    }

    pub fn inverse_patheticism(&self) -> f64 {
        todo!()
    }

    pub fn muscularity(&self) -> f64 {
        todo!()
    }

    pub fn divinity(&self) -> f64 {
        todo!()
    }

    pub fn moxie(&self) -> f64 {
        todo!()
    }
}
