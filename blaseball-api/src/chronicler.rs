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

pub fn versions<ItemType: DeserializeOwned + Debug>(
    entity_type: &'static str,
    start: DateTime<Utc>,
) -> impl Stream<Item = ChroniclerItem<ItemType>> {
    chronicler_pages("versions", entity_type, None, start)
        .flat_map(|vec| stream::iter(vec.into_iter()))
}

pub fn entities<ItemType: DeserializeOwned + Debug>(
    entity_type: &'static str,
    start: DateTime<Utc>,
) -> impl Stream<Item = ChroniclerItem<ItemType>> {
    chronicler_pages("entities", entity_type, None, start)
        .flat_map(|vec| stream::iter(vec.into_iter()))
}

pub async fn team(team_id: Uuid, at_time: DateTime<Utc>) -> Option<ChroniclerItem<ChroniclerTeam>> {
    let ids = [team_id];
    let mut stream = pin!(chronicler_pages("entities", "team", Some(&ids), at_time));

    stream
        .next()
        .await
        .and_then(|items| items.into_iter().next())
}

pub async fn player(
    player_id: Uuid,
    at_time: DateTime<Utc>,
) -> Option<ChroniclerItem<ChroniclerPlayer>> {
    let ids = [player_id];
    let mut stream = pin!(chronicler_pages("entities", "player", Some(&ids), at_time));

    stream
        .next()
        .await
        .and_then(|items| items.into_iter().next())
}

pub fn game_updates(start: DateTime<Utc>) -> impl Stream<Item = ChroniclerGameUpdate> {
    game_update_pages(start).flat_map(|vec| stream::iter(vec.into_iter()))
}

struct ChronState {
    pub page: Option<String>,
    pub stop: bool,
    pub cache: sled::Db,
    pub client: reqwest::Client,
}

fn chronicler_pages<'ids, ItemType: DeserializeOwned + Debug>(
    endpoint: &'static str,
    entity_type: &'static str,
    entity_ids: Option<&'ids [Uuid]>,
    start: DateTime<Utc>,
) -> impl Stream<Item = Vec<ChroniclerItem<ItemType>>> + use<'ids, ItemType> {
    let start_state = ChronState {
        page: None,
        stop: false,
        cache: sled::open("http_cache/chron/".to_owned() + endpoint + "/" + entity_type).unwrap(),
        client: reqwest::Client::new(),
    };

    stream::unfold(start_state, move |state| async move {
        if state.stop {
            None
        } else {
            Some(chronicler_page(start, endpoint, entity_type, entity_ids, state).await)
        }
    })
}

async fn chronicler_page<ItemType: DeserializeOwned + Debug>(
    start: DateTime<Utc>,
    endpoint: &'static str,
    entity_type: &'static str,
    entity_ids: Option<&[Uuid]>,
    state: ChronState,
) -> (Vec<ChroniclerItem<ItemType>>, ChronState) {
    let request = state
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
    let response = match state.cache.get(&cache_key).unwrap() {
        Some(text) => {
            info!("Loading {cache_key} from cache");
            bincode::deserialize(&text).unwrap()
        }
        None => {
            info!("Fetching {cache_key} from Chron");
            let text = state
                .client
                .execute(request)
                .await
                .expect("Chronicler API call failed")
                .text()
                .await
                .expect("Chronicler text decode failed");

            state
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
        ChronState {
            page: response.next_page,
            stop,
            cache: state.cache,
            client: state.client,
        },
    )
}

fn game_update_pages(start: DateTime<Utc>) -> impl Stream<Item = Vec<ChroniclerGameUpdate>> {
    let start_state = ChronState {
        page: None,
        stop: false,
        cache: sled::open("http_cache/game/updates".to_string()).unwrap(),
        client: reqwest::Client::new(),
    };

    stream::unfold(start_state, move |state| async move {
        if state.stop {
            None
        } else {
            Some(game_update_page(start, state).await)
        }
    })
}

async fn game_update_page(
    start: DateTime<Utc>,
    state: ChronState,
) -> (Vec<ChroniclerGameUpdate>, ChronState) {
    let request = state
        .client
        .get("https://api.sibr.dev/chronicler/v1/games/updates".to_string())
        .query(&[("after", &start)]);

    let request = match state.page {
        Some(page) => request.query(&[("page", &page)]),
        None => request,
    };

    let request = request.build().unwrap();

    let cache_key = request.url().to_string();
    let response = match state.cache.get(&cache_key).unwrap() {
        Some(text) => bincode::deserialize(&text).unwrap(),
        None => {
            info!("Fetching game updates page from network");

            let text = state
                .client
                .execute(request)
                .await
                .expect("Chronicler API call failed")
                .text()
                .await
                .expect("Chronicler text decode failed");

            state
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
        ChronState {
            page: response.next_page,
            stop,
            cache: state.cache,
            client: state.client,
        },
    )
}
