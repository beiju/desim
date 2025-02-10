use crate::rng::Rng;
use chrono::{DateTime, Utc};
use flate2::bufread::GzDecoder;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read};
use tar::Archive;
use thiserror::Error;

pub type Fragments = Vec<Fragment>;

// This is the object the rest of desim deals with
#[derive(Debug, Clone)]
pub struct Fragment {
    pub label: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub rng: Rng,
    pub check_rolls: Option<RollStream>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckRoll {
    pub label: String,
    pub roll: f64,
    pub passed: Option<bool>,
    pub threshold: Option<f64>,
}

#[derive(Debug, Error)]
pub enum LoadFragmentsError {
    #[error("Corrupted fragments file: {0}")]
    CorruptedFragmentsFile(json5::Error),

    #[error("Corrupted roll streams archive: {0}")]
    CorruptedRollStreamsArchive(std::io::Error),

    // The "or was specified multiple times" is because we remove the RollStream
    // the first time we encounter it, so if we encounter it again it'll look
    // like it was missing the whole time
    #[error("Roll stream for {0} was not found in bundled roll stream archive, or was specified multiple times")]
    MissingRollStream(String),

    #[error("Invalid JSON in roll streams archive: {0}")]
    InvalidJsonInRollStreamsArchive(serde_json::Error),
}

// This is what we deserialize from disk
#[derive(Debug, Clone, Deserialize)]
struct FragmentSpec {
    pub label: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub rng: Rng,
    pub initial_step: Option<i32>,
    pub roll_stream: Option<RollStreamSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct RollStreamSpec {
    pub file: String,
    pub skip_lines: Option<usize>,
}

pub type RollStream = VecDeque<CheckRoll>;

pub fn load_fragments() -> Result<Fragments, LoadFragmentsError> {
    let fragments_json5 = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/fragments.json5"
    ));

    let fragments_specs = json5::from_str::<Vec<FragmentSpec>>(fragments_json5)
        .map_err(LoadFragmentsError::CorruptedFragmentsFile)?;

    let streams_to_load = fragments_specs
        .iter()
        .flat_map(|spec| &spec.roll_stream)
        .map(|stream| (stream.file.clone(), stream.skip_lines.unwrap_or(0)))
        .collect::<HashMap<_, _>>();

    println!("Want to load {} roll streams", streams_to_load.len());

    let mut roll_streams = load_roll_streams(streams_to_load)?;
    let fragments = fragments_specs
        .into_iter()
        .map(|spec| fragment_from_spec(spec, &mut roll_streams))
        .collect::<Result<_, _>>()?;

    Ok(fragments)
}

fn load_roll_streams(
    mut streams_to_load: HashMap<String, usize>,
) -> Result<HashMap<String, RollStream>, LoadFragmentsError> {
    let raw_data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/roll_streams.tar.gz"
    ));
    let gzip_decoder = GzDecoder::new(&raw_data[..]);
    let mut streams = Archive::new(gzip_decoder);
    println!("Loading roll streams");
    let roll_streams = streams
        .entries()
        .map_err(LoadFragmentsError::CorruptedRollStreamsArchive)?
        .map(|entry| {
            println!("In roll streams loader");
            let entry = entry.map_err(LoadFragmentsError::CorruptedRollStreamsArchive)?;

            let path = entry
                .path()
                .map_err(LoadFragmentsError::CorruptedRollStreamsArchive)?;

            println!("Encountered \"{}\" in roll streams archive", path.display());
            if let Some(path_str) = path.to_str() {
                if let Some((key, skip_lines)) = streams_to_load.remove_entry(path_str) {
                    println!("Loading \"{}\"", path_str);
                    return Ok(Some((key, load_roll_stream(entry, skip_lines)?)));
                }
            }

            Ok::<_, LoadFragmentsError>(None)
        })
        .flatten_ok()
        .collect::<Result<_, _>>()?;

    Ok(roll_streams)
}

fn load_roll_stream(entry: impl Read, skip_lines: usize) -> Result<RollStream, LoadFragmentsError> {
    BufReader::new(entry)
        .lines()
        .skip(skip_lines)
        .map(|line| {
            let line = line.map_err(|e| LoadFragmentsError::CorruptedRollStreamsArchive(e))?;

            serde_json::from_str(&line)
                .map_err(|e| LoadFragmentsError::CorruptedRollStreamsArchive(e.into()))
        })
        .collect()
}

fn fragment_from_spec(
    spec: FragmentSpec,
    roll_streams: &mut HashMap<String, RollStream>,
) -> Result<Fragment, LoadFragmentsError> {
    let mut rng = spec.rng;
    if let Some(step_by) = spec.initial_step {
        rng.step(step_by);
    }
    Ok(Fragment {
        label: spec.label,
        start_time: spec.start_time,
        end_time: spec.end_time,
        rng,
        check_rolls: spec
            .roll_stream
            .map(|s| get_roll_stream(s, roll_streams))
            .transpose()?,
    })
}

fn get_roll_stream(
    spec: RollStreamSpec,
    roll_streams: &mut HashMap<String, RollStream>,
) -> Result<RollStream, LoadFragmentsError> {
    roll_streams
        .remove(spec.file.as_str())
        .ok_or_else(|| LoadFragmentsError::MissingRollStream(spec.file))
}
