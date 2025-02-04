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

pub fn game_updates_or_schedule(
    schedule: bool,
    start: DateTime<Utc>,
) -> impl Stream<Item = ChroniclerItem> {
    game_update_pages(schedule, start)
        .flat_map(|vec| stream::iter(vec.into_iter()))
        .map(|game| ChroniclerItem {
            entity_id: game.game_id,
            valid_from: game.timestamp,
            valid_to: None,
            data: game.data,
        })
}

pub fn game_updates(start: DateTime<Utc>) -> impl Stream<Item = ChroniclerItem> {
    game_updates_or_schedule(false, start)
}

pub fn schedule(start: DateTime<Utc>) -> impl Stream<Item = ChroniclerItem> {
    game_updates_or_schedule(true, start)
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

fn game_update_pages(
    schedule: bool,
    start: DateTime<Utc>,
) -> impl Stream<Item = Vec<ChroniclerGameUpdate>> {
    let request_type = if schedule { "schedule" } else { "updates" };

    let start_state = ChronState {
        page: None,
        stop: false,
        cache: sled::open("http_cache/game/".to_string() + request_type).unwrap(),
        client: reqwest::Client::new(),
    };

    stream::unfold(start_state, move |state| async move {
        if state.stop {
            None
        } else {
            Some(game_update_page(schedule, start, state).await)
        }
    })
}

async fn game_update_page(
    schedule: bool,
    start: DateTime<Utc>,
    state: ChronState,
) -> (Vec<ChroniclerGameUpdate>, ChronState) {
    let request_type = if schedule { "schedule" } else { "updates" };

    let request = state
        .client
        .get(
            "https://api.sibr.dev/chronicler/v1/games".to_string()
                + if schedule { "" } else { "/updates" },
        )
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
            info!("Fetching game {} page from network", request_type);

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

    let client = &state.client;
    let cache = &state.cache;
    let (response_data, next_page) = if schedule {
        let games_response: ChroniclerGamesResponse = serde_json::from_str(&response).unwrap();
        let games: Vec<_> = stream::iter(games_response.data.into_iter())
            .then(move |item| async move {
                let request = client
                    .get("https://api.sibr.dev/chronicler/v1/games/updates")
                    .query(&[("game", item.game_id.to_string())])
                    .query(&[("order", "asc")])
                    .query(&[("count", 1)])
                    .build()
                    .unwrap();

                let cache_key = request.url().to_string();
                let response = match cache.get(&cache_key).unwrap() {
                    Some(text) => bincode::deserialize(&text).unwrap(),
                    None => {
                        info!(
                            "Fetching latest update for game {} from network",
                            item.game_id
                        );

                        let text = client
                            .execute(request)
                            .await
                            .expect("Chronicler API call failed")
                            .text()
                            .await
                            .expect("Chronicler text decode failed");

                        cache
                            .insert(&cache_key, bincode::serialize(&text).unwrap())
                            .unwrap();

                        text
                    }
                };

                let response: ChroniclerGameUpdatesResponse =
                    serde_json::from_str(&response).unwrap();

                response.data.into_iter().next().unwrap()
            })
            .collect()
            .await;

        (games, games_response.next_page)
    } else {
        let response: ChroniclerGameUpdatesResponse = serde_json::from_str(&response).unwrap();
        (response.data, response.next_page)
    };

    let stop = next_page.is_none();
    (
        response_data,
        ChronState {
            page: next_page,
            stop,
            cache: state.cache,
            client: state.client,
        },
    )
}
