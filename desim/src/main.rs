#[macro_use]
extern crate rocket;
mod checker;
mod engine;
mod fragments;
mod rng;
mod rolls;
mod sim;
mod thresholds;
mod update_parser;

use crate::engine::{Engine, EngineFatalError};
use crate::fragments::{load_fragments, Fragments};
use crate::thresholds::Thresholds;
use blaseball_api::chronicler;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use rocket::futures::StreamExt;
use rocket::http::uri::fmt::{Formatter, FromUriParam, UriDisplay};
use rocket::request::FromParam;
use rocket::{response, Request, Response};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::future;
use std::pin::pin;
use thiserror::Error;

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum RollConstraintOutcome {
    /// Succeeded because we don't know enough to confirm that it failed
    TrivialSuccess,
    /// We're confident that this roll obeyed the threshold
    Success,
    /// We're confident that this roll violated the threshold
    Failure,
    /// We're confident that the sim doesn't use the outcome of this roll
    Unused,
}

#[derive(Error, Debug)]
pub enum DesimError {
    #[error("Could not find fragment {0}")]
    UnknownFragment(usize),

    #[error("Couldn't deserialize game: {0:?}")]
    DeserializeGameFailed(serde_json::Error),

    #[error("There were no game events on this day")]
    NoGameEventsThisDay,

    #[error(transparent)]
    EngineError(#[from] EngineFatalError),
}

impl<'r, 'o: 'r> response::Responder<'r, 'o> for DesimError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        #[derive(Serialize)]
        struct ErrContext<'a> {
            header: &'a str,
            body: Option<String>,
        }

        let context = match self {
            DesimError::UnknownFragment(err) => ErrContext {
                header: "Nonexistent fragment requested",
                body: Some(err.to_string()),
            },
            DesimError::DeserializeGameFailed(err) => ErrContext {
                header: "Failed to deserialize game",
                body: Some(err.to_string()),
            },
            DesimError::NoGameEventsThisDay => ErrContext {
                header: "No game events this day",
                body: None,
            },
            DesimError::EngineError(err) => ErrContext {
                header: "Engine error",
                body: Some(err.to_string()),
            },
        };
        let template = Template::render("error", context);
        Response::build_from(template.respond_to(req)?).ok()
    }
}

struct DateTimeParam(DateTime<Utc>);

impl<'a> FromParam<'a> for DateTimeParam {
    type Error = chrono::ParseError;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Ok(Self(DateTime::parse_from_rfc3339(param)?.to_utc()))
    }
}

impl UriDisplay<rocket::http::uri::fmt::Path> for DateTimeParam {
    fn fmt(&self, f: &mut Formatter<'_, rocket::http::uri::fmt::Path>) -> std::fmt::Result {
        UriDisplay::fmt(&self.0.to_rfc3339(), f)
    }
}

impl FromUriParam<rocket::http::uri::fmt::Path, &DateTime<Utc>> for DateTimeParam {
    type Target = DateTimeParam;

    fn from_uri_param(param: &DateTime<Utc>) -> Self::Target {
        DateTimeParam(*param)
    }
}

#[get("/")]
fn index(fragments: &rocket::State<Fragments>) -> Template {
    #[derive(Serialize)]
    struct FragmentContext<'route> {
        title: &'route str,
        uri: rocket::http::uri::Origin<'route>,
    }

    let fragments_view = fragments
        .iter()
        .enumerate()
        .map(|(fragment_index, fragment)| FragmentContext {
            title: &fragment.label,
            uri: uri!(fragment(fragment_index)),
        })
        .collect_vec();

    Template::render(
        "index",
        context! {
            fragments: fragments_view,
        },
    )
}

#[get("/fragment/<fragment_index>")]
async fn fragment(
    fragment_index: usize,
    fragments: &rocket::State<Fragments>,
    th: &rocket::State<Thresholds>,
) -> Result<Template, DesimError> {
    let fragment = fragments
        .get(fragment_index)
        .ok_or(DesimError::UnknownFragment(fragment_index))?;

    // Gather data
    let mut game_updates = pin!(chronicler::game_updates(fragment.start_time)
        .take_while(|update| future::ready(update.timestamp < fragment.end_time)));

    // Start the engine
    let mut engine = Engine::new(fragment.rng.clone());

    let mut days = Vec::new();
    while let Some(update) = game_updates.next().await {
        debug!(
            "Processing game update for s{}d{} game {} at time {}",
            update.data.season + 1,
            update.data.day + 1,
            update.data.id,
            update.timestamp,
        );
        if let Some(new_day) = engine.next_update(update, th).await? {
            days.push(new_day);
        }
    }

    Ok(Template::render(
        "fragment",
        context! {
            fragment_label: &fragment.label,
            days: days,
        },
    ))
}

#[launch]
fn rocket() -> _ {
    let fragments = load_fragments().expect("Failed to load fragments");
    let th = Thresholds::load().expect("Failed to load thresholds");

    let static_path = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
    rocket::build()
        .manage(fragments)
        .manage(th)
        .mount("/static", rocket::fs::FileServer::from(static_path))
        .mount("/", routes![index, fragment])
        .attach(Template::fairing())
}
