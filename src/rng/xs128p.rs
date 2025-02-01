#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Xs128pState(pub u64, pub u64);

impl From<(u64, u64)> for Xs128pState {
    fn from(value: (u64, u64)) -> Self {
        return Xs128pState(value.0, value.1);
    }
}

impl Into<(u64, u64)> for Xs128pState {
    fn into(self) -> (u64, u64) {
        return (self.0, self.1);
    }
}

pub fn xs128p(state: Xs128pState) -> Xs128pState {
    let (mut s1, s0) = (state.0, state.1);
    s1 ^= s1 << 23;
    s1 ^= s1 >> 17;
    s1 ^= s0;
    s1 ^= s0 >> 26;
    Xs128pState(state.1, s1)
}

fn reverse17(val: u64) -> u64 {
    val ^ (val >> 17) ^ (val >> 34) ^ (val >> 51)
}

fn reverse23(val: u64) -> u64 {
    val ^ (val << 23) ^ (val << 46)
}

pub fn xs128p_rev(state: Xs128pState) -> Xs128pState {
    let prev_state1 = state.0;
    let mut prev_state0 = state.1 ^ (state.0 >> 26);
    prev_state0 ^= state.0;
    prev_state0 = reverse17(prev_state0);
    prev_state0 = reverse23(prev_state0);
    Xs128pState(prev_state0, prev_state1)
}

pub fn from_double_bits(bits: u64) -> f64 {
    let mantissa = bits & ((1u64 << 52) - 1);
    let full = mantissa | 0x3FF0000000000000;
    f64::from_bits(full) - 1.0
}

pub fn from_double_bits_v10(s0: u64, s1: u64) -> f64 {
    let sum = s0.wrapping_add(s1);
    let full = (sum & 0x000FFFFFFFFFFFFF) | 0x3FF0000000000000;
    f64::from_bits(full) - 1.0
}

pub fn to_double_bits(val: f64) -> u64 {
    let bits = (val + 1.0).to_bits();
    return bits & ((1 << 52) - 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_fwd() {
        // Kichiro Guerra (thwack ->) lastName roll
        assert_eq!(
            xs128p(Xs128pState(11489856334623440466, 7665746933450455135)),
            Xs128pState(7665746933450455135, 5757515306888244331)
        );

        // Miguel Javier (thwack ->) lastName roll
        assert_eq!(
            xs128p(Xs128pState(4278828314640535865, 3539470500018873972)),
            Xs128pState(3539470500018873972, 15939708256490641700)
        );
    }

    #[test]
    fn basic_rev() {
        // Kichiro Guerra (thwack ->) moxie roll
        assert_eq!(
            xs128p_rev(Xs128pState(11489856334623440466, 7665746933450455135)),
            Xs128pState(3568317142935851365, 11489856334623440466)
        );

        // Miguel Javier (thwack ->) moxie roll
        assert_eq!(
            xs128p_rev(Xs128pState(4278828314640535865, 3539470500018873972)),
            Xs128pState(4918252522610053827, 4278828314640535865)
        );
    }
}
