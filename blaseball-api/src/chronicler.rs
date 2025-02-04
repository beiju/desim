use crate::chronicler_schema::{
    ChroniclerGameUpdate, ChroniclerGameUpdatesResponse, ChroniclerGamesResponse, ChroniclerItem,
    ChroniclerResponse,
};
use chrono::{DateTime, Utc};
use futures::{stream, Stream, StreamExt};
use log::info;

pub fn versions(
    entity_type: &'static str,
    start: DateTime<Utc>,
) -> impl Stream<Item = ChroniclerItem> {
    chronicler_pages("versions", entity_type, start).flat_map(|vec| stream::iter(vec.into_iter()))
}

pub fn entities(
    entity_type: &'static str,
    start: DateTime<Utc>,
) -> impl Stream<Item = ChroniclerItem> {
    chronicler_pages("entities", entity_type, start).flat_map(|vec| stream::iter(vec.into_iter()))
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

fn chronicler_pages(
    endpoint: &'static str,
    entity_type: &'static str,
    start: DateTime<Utc>,
) -> impl Stream<Item = Vec<ChroniclerItem>> {
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
            Some(chronicler_page(start, endpoint, entity_type, state).await)
        }
    })
}

async fn chronicler_page(
    start: DateTime<Utc>,
    endpoint: &'static str,
    entity_type: &'static str,
    state: ChronState,
) -> (Vec<ChroniclerItem>, ChronState) {
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
        Some(page) => request.query(&[("page", &page)]),
        None => request,
    };

    let request = request.build().unwrap();

    let cache_key = request.url().to_string();
    let response = match state.cache.get(&cache_key).unwrap() {
        Some(text) => bincode::deserialize(&text).unwrap(),
        None => {
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

    let response: ChroniclerResponse = serde_json::from_str(&response).unwrap();

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
