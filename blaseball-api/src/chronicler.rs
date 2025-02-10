use crate::chronicler_schema::{
    ChroniclerGameUpdate, ChroniclerGameUpdatesResponse, ChroniclerItem, ChroniclerPlayer,
    ChroniclerResponse, ChroniclerTeam,
};
use chrono::{DateTime, Utc};
use futures::{stream, Stream, StreamExt};
use itertools::Itertools;
use log::info;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::pin::pin;
use uuid::Uuid;

pub struct Chronicler {
    cache: sled::Db,
    client: reqwest::Client,
}

struct ChronPaginationState {
    pub page: Option<String>,
    pub stop: bool,
}

impl Chronicler {
    pub fn new() -> sled::Result<Self> {
        Ok(Chronicler {
            cache: sled::open("http_cache/chron/".to_owned())?,
            client: reqwest::Client::new(),
        })
    }

    pub fn versions<ItemType: DeserializeOwned + Debug>(
        &self,
        entity_type: &'static str,
        start: DateTime<Utc>,
    ) -> impl Stream<Item = ChroniclerItem<ItemType>> + use<'_, ItemType> {
        self.chronicler_pages("versions", entity_type, None, start)
            .flat_map(|vec| stream::iter(vec.into_iter()))
    }

    pub fn entities<ItemType: DeserializeOwned + Debug>(
        &self,
        entity_type: &'static str,
        start: DateTime<Utc>,
    ) -> impl Stream<Item = ChroniclerItem<ItemType>> + use<'_, ItemType> {
        self.chronicler_pages("entities", entity_type, None, start)
            .flat_map(|vec| stream::iter(vec.into_iter()))
    }

    pub async fn team(&self, team_id: Uuid, at_time: DateTime<Utc>) -> Option<ChroniclerItem<ChroniclerTeam>> {
        let ids = [team_id];
        let mut stream = pin!(self.chronicler_pages("entities", "team", Some(&ids), at_time));

        stream
            .next()
            .await
            .and_then(|items| items.into_iter().next())
    }

    pub async fn player(
        &self,
        player_id: Uuid,
        at_time: DateTime<Utc>,
    ) -> Option<ChroniclerItem<ChroniclerPlayer>> {
        let ids = [player_id];
        let mut stream = pin!(self.chronicler_pages("entities", "player", Some(&ids), at_time));

        stream
            .next()
            .await
            .and_then(|items| items.into_iter().next())
    }

    pub fn game_updates(&self, start: DateTime<Utc>) -> impl Stream<Item = ChroniclerGameUpdate> + use<'_> {
        self.game_update_pages(start).flat_map(|vec| stream::iter(vec.into_iter()))
    }

    fn chronicler_pages<'ids, 's, ItemType: DeserializeOwned + Debug>(
        &'s self,
        endpoint: &'static str,
        entity_type: &'static str,
        entity_ids: Option<&'ids [Uuid]>,
        start: DateTime<Utc>,
    ) -> impl Stream<Item = Vec<ChroniclerItem<ItemType>>> + use<'ids, 's, ItemType> {
        let start_state = ChronPaginationState {
            page: None,
            stop: false,
        };

        stream::unfold(start_state, move |state| async move {
            if state.stop {
                None
            } else {
                Some(self.chronicler_page(start, endpoint, entity_type, entity_ids, state).await)
            }
        })
    }

    async fn chronicler_page<ItemType: DeserializeOwned + Debug>(
        &self,
        start: DateTime<Utc>,
        endpoint: &'static str,
        entity_type: &'static str,
        entity_ids: Option<&[Uuid]>,
        state: ChronPaginationState,
    ) -> (Vec<ChroniclerItem<ItemType>>, ChronPaginationState) {
        let request = self
            .client
            .get("https://api.sibr.dev/chronicler/v2/".to_owned() + endpoint)
            .query(&[("type", &entity_type)]);

        let request = match endpoint {
            "entities" => request.query(&[("at", &start)]),
            "versions" => request.query(&[("after", &start)]),
            _ => panic!("Unexpected endpoint: {}", endpoint),
        };

        let request = match state.page {
            None => request,
            Some(page) => request.query(&[("page", &page)]),
        };

        let request = match entity_ids {
            None => request,
            Some(ids) => request.query(&[("id", ids.iter().map(|id| id.to_string()).join(","))]),
        };

        let request = request.build().unwrap();

        let cache_key = request.url().to_string();
        let response = match self.cache.get(&cache_key).unwrap() {
            Some(text) => {
                info!("Loading {cache_key} from cache");
                bincode::deserialize(&text).unwrap()
            }
            None => {
                info!("Fetching {cache_key} from Chron");
                let text = self
                    .client
                    .execute(request)
                    .await
                    .expect("Chronicler API call failed")
                    .text()
                    .await
                    .expect("Chronicler text decode failed");

                self
                    .cache
                    .insert(&cache_key, bincode::serialize(&text).unwrap())
                    .unwrap();

                text
            }
        };

        let response: ChroniclerResponse<ItemType> = serde_json::from_str(&response).unwrap();

        let stop = response.next_page.is_none();
        (
            response.items,
            ChronPaginationState {
                page: response.next_page,
                stop,
            },
        )
    }

    fn game_update_pages(&self, start: DateTime<Utc>) -> impl Stream<Item=Vec<ChroniclerGameUpdate>> + use<'_> {
        let start_state = ChronPaginationState {
            page: None,
            stop: false,
        };

        stream::unfold(start_state, move |state| async move {
            if state.stop {
                None
            } else {
                Some(self.game_update_page(start, state).await)
            }
        })
    }

    async fn game_update_page(
        &self,
        start: DateTime<Utc>,
        state: ChronPaginationState,
    ) -> (Vec<ChroniclerGameUpdate>, ChronPaginationState) {
        let request = self
            .client
            .get("https://api.sibr.dev/chronicler/v1/games/updates".to_string())
            .query(&[("after", &start)]);

        let request = match state.page {
            Some(page) => request.query(&[("page", &page)]),
            None => request,
        };

        let request = request.build().unwrap();

        let cache_key = request.url().to_string();
        let response = match self.cache.get(&cache_key).unwrap() {
            Some(text) => bincode::deserialize(&text).unwrap(),
            None => {
                info!("Fetching game updates page from network");

                let text = self
                    .client
                    .execute(request)
                    .await
                    .expect("Chronicler API call failed")
                    .text()
                    .await
                    .expect("Chronicler text decode failed");

                self
                    .cache
                    .insert(&cache_key, bincode::serialize(&text).unwrap())
                    .unwrap();

                text
            }
        };

        let response: ChroniclerGameUpdatesResponse = serde_json::from_str(&response).unwrap();

        let stop = response.next_page.is_none();
        (
            response.data,
            ChronPaginationState {
                page: response.next_page,
                stop,
            },
        )
    }


}
