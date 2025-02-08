mod xs128p;

use nom::Finish;
use serde::{Deserialize, Deserializer};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use xs128p::{from_double_bits, from_double_bits_v10, xs128p, xs128p_rev, Xs128pState};

type BlockOffset = i32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rng {
    pub state: Xs128pState,
    pub offset: BlockOffset,
    pub v10: bool,
}

impl Display for Rng {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}+{}", self.state.0, self.state.1, self.offset)
    }
}

#[derive(Debug, Error)]
pub enum RngDeserializeError<'s> {
    #[error("Failed to parse RNG string: {0}")]
    ParseError(nom::error::Error<&'s str>),
    #[error("Failed to parse int within RNG string: {0}")]
    ParseIntError(std::num::ParseIntError),
}

fn parse_rng_str_helper(
    input: &str,
) -> Result<((&str, &str), &str), nom::Err<nom::error::Error<&str>>> {
    use nom::{
        bytes::complete::tag,
        character::complete::digit1,
        combinator::{eof, opt},
        Parser,
    };
    let (input, _) = tag("(").parse(input)?;
    let (input, s0) = digit1.parse(input)?;
    let (input, _) = tag(",").parse(input)?;
    let (input, _) = opt(tag(" ")).parse(input)?;
    let (input, s1) = digit1.parse(input)?;
    let (input, _) = tag(")+").parse(input)?;
    let (input, o) = digit1.parse(input)?;
    let (_, _) = eof.parse(input)?;

    Ok(((s0, s1), o))
}

fn rng_from_strs(s0: &str, s1: &str, o: &str) -> Result<Rng, std::num::ParseIntError> {
    let s0 = s0.parse()?;
    let s1 = s1.parse()?;
    let o = o.parse()?;

    Ok(Rng::new((s0, s1), o))
}

fn parse_rng_str(s: &str) -> Result<Rng, RngDeserializeError> {
    let ((s0, s1), o) = parse_rng_str_helper(s)
        .finish()
        .map_err(|e| RngDeserializeError::ParseError(e))?;
    let rng = rng_from_strs(s0, s1, o).map_err(|e| RngDeserializeError::ParseIntError(e))?;
    Ok(rng)
}

impl<'de> Deserialize<'de> for Rng {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        // let s: &str will compile, but error at runtime. I'm not pleased that
        // compiles-but-errors-at-runtime is a consequence of incorrect types.
        // Isn't that the whole point of static typing
        let s: String = Deserialize::deserialize(deserializer)?;
        parse_rng_str(&s).map_err(D::Error::custom)
    }
}

pub fn normalize_offset(offset: BlockOffset) -> BlockOffset {
    return offset.rem_euclid(64);
}

pub fn calculate_steps(
    current_offset: BlockOffset,
    requested_steps: i32,
    block_size: i32,
) -> (i32, i32) {
    let total_blocks = -(current_offset - requested_steps).div_euclid(block_size);

    let block_steps = total_blocks * block_size * 2;
    let total_steps = block_steps - requested_steps;

    let new_offset = (current_offset - requested_steps).rem_euclid(block_size);

    (total_steps, new_offset)
}

// pub const CHECKPOINT_BITS: u64 = 16;
pub const CHECKPOINT_BITS: u64 = 20;
pub const CHECKPOINT_MASK: u64 = (1 << CHECKPOINT_BITS) - 1;
pub fn is_checkpoint(state: Xs128pState) -> bool {
    (state.0 & CHECKPOINT_MASK) == 0
}

pub fn is_checkpoint_bits(state: Xs128pState, bits: usize) -> bool {
    let mask = (1 << bits) - 1;
    (state.0 & mask) == 0
}

pub fn find_checkpoint(state: Xs128pState, offset: i32) -> (Xs128pState, i32, i32) {
    find_checkpoint_bits(state, offset, 16)
}

pub fn find_checkpoint_bits(
    state: Xs128pState,
    offset: i32,
    bits: usize,
) -> (Xs128pState, i32, i32) {
    let mut distance = 0;

    let mut rng = Rng::new(state, offset);
    while !is_checkpoint_bits(rng.state, bits) {
        rng.step(-1);
        // state = xs128p_rev(state);
        distance += 1;
    }
    (rng.state, distance, rng.offset)
}

impl Rng {
    pub fn new(state: impl Into<Xs128pState>, offset: BlockOffset) -> Rng {
        Rng {
            state: state.into(),
            offset: normalize_offset(offset),
            v10: false,
        }
    }

    pub fn state_tuple(&self) -> (u64, u64, BlockOffset) {
        (self.state.0, self.state.1, self.offset)
    }

    pub fn step_raw(&mut self, steps: i32) {
        if steps > 0 {
            for _ in 0..steps {
                self.state = xs128p(self.state);
            }
        } else {
            for _ in 0..(-steps) {
                self.state = xs128p_rev(self.state);
            }
        }
    }

    pub fn step(&mut self, steps: i32) {
        let block_size = if self.v10 { 62 } else { 64 };
        let (total_steps, new_offset) = calculate_steps(self.offset, steps, block_size);
        self.step_raw(total_steps);
        self.offset = new_offset;
    }

    pub fn seek_prev_checkpoint(&mut self, bits: usize) -> usize {
        self.step(-1);

        let mut steps = 1;
        while !is_checkpoint_bits(self.state, bits) {
            self.step(-1);
            steps += 1;
        }
        steps
    }

    pub fn seek_next_checkpoint(&mut self, bits: usize) {
        self.step(1);

        while !is_checkpoint_bits(self.state, bits) {
            self.step(1);
        }
    }

    pub fn value(&self) -> f64 {
        if self.v10 {
            from_double_bits_v10(self.state.0, self.state.1)
        } else {
            from_double_bits(self.state.0 >> 12)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(before: (u64, u64, BlockOffset), steps: BlockOffset) -> (u64, u64, BlockOffset) {
        let mut rng = Rng::new(Xs128pState(before.0, before.1), before.2);
        rng.step(steps);
        rng.state_tuple()
    }

    #[test]
    fn test() {
        let start = (11489856334623440466, 7665746933450455135, 59);

        // Basics
        assert_eq!(
            step(start, 1),
            (3568317142935851365, 11489856334623440466, 58)
        );
        assert_eq!(
            step(start, -1),
            (7665746933450455135, 5757515306888244331, 60)
        );
        assert_eq!(
            step(start, 2),
            (1871981691294829610, 3568317142935851365, 57)
        );
        assert_eq!(
            step(start, -4),
            (11777078382307459003, 9189176605564379358, 63)
        );
        assert_eq!(
            step(start, 32),
            (3267963782523076449, 2615119604951746693, 27)
        );

        // Crossing block boundaries
        assert_eq!(
            step(start, 59),
            (4418950297936233643, 8461946988962992193, 0)
        );
        assert_eq!(
            step(start, 60),
            (9595792334013182699, 8659343871044683043, 63)
        );
        assert_eq!(
            step(start, -5),
            (7350346046143330015, 15192697735018323666, 0)
        );

        // Stepping a full block
        assert_eq!(
            step(start, 64),
            (3433578427688570473, 2440012305804807291, 59)
        );
        assert_eq!(
            step(start, -64),
            (656475616170205904, 5053579426408536524, 59)
        );

        // Stepping over blocks
        assert_eq!(
            step(start, 128),
            (15955351200758865640, 14106346560409878372, 59)
        );
        assert_eq!(
            step(start, -128),
            (14437018569946036092, 16257786924949580527, 59)
        );
        assert_eq!(
            step(start, 127),
            (14106346560409878372, 12559088209872134966, 60)
        );
        assert_eq!(
            step(start, -127),
            (12479537282219661871, 14437018569946036092, 58)
        );

        assert_eq!(
            step(start, 59 + 64),
            (9189176605564379358, 17219780032394536164, 0)
        );
        assert_eq!(
            step(start, 59 + 64 + 1),
            (7974845343091599361, 8534881269550711784, 63)
        );
        assert_eq!(
            step(start, -5 - 64 + 1),
            (8453525309065067247, 4418950297936233643, 63)
        );
        assert_eq!(
            step(start, -5 - 64),
            (9524146849697370050, 12966572773286726302, 0)
        );

        // Stepping really far
        assert_eq!(
            step(start, 3000),
            (7423595971207329334, 16910322575388945665, 3)
        );
        assert_eq!(
            step(start, -3000),
            (5559434767711380194, 12515405342771602967, 51)
        );
    }
}
