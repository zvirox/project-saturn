use serde::{Deserialize, Serialize};

pub const TICKS_PER_SECOND: i64 = 254_016_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tick(pub i64);

impl Tick {
    pub fn from_nanoseconds(nanoseconds: u64) -> Option<Self> {
        let ticks = i128::from(nanoseconds)
            .checked_mul(i128::from(TICKS_PER_SECOND))?
            .checked_add(500_000_000)?
            / 1_000_000_000;
        i64::try_from(ticks).ok().map(Self)
    }

    pub fn as_nanoseconds(self) -> Option<u64> {
        if self.0 < 0 {
            return None;
        }
        let nanoseconds = i128::from(self.0)
            .checked_mul(1_000_000_000)?
            .checked_add(i128::from(TICKS_PER_SECOND / 2))?
            / i128::from(TICKS_PER_SECOND);
        u64::try_from(nanoseconds).ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub const FPS_24: Self = Self {
        numerator: 24,
        denominator: 1,
    };
    pub const FPS_25: Self = Self {
        numerator: 25,
        denominator: 1,
    };
    pub const FPS_30: Self = Self {
        numerator: 30,
        denominator: 1,
    };
    pub const FPS_23976: Self = Self {
        numerator: 24_000,
        denominator: 1_001,
    };
    pub const FPS_2997: Self = Self {
        numerator: 30_000,
        denominator: 1_001,
    };

    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        (numerator > 0 && denominator > 0).then_some(Self {
            numerator,
            denominator,
        })
    }

    pub fn frame_ticks(self) -> Option<i64> {
        if self.numerator == 0 || self.denominator == 0 {
            return None;
        }
        let value = i128::from(TICKS_PER_SECOND).checked_mul(i128::from(self.denominator))?;
        if value % i128::from(self.numerator) != 0 {
            return None;
        }
        i64::try_from(value / i128::from(self.numerator)).ok()
    }

    pub fn frames_to_ticks(self, frames: i64) -> Option<Tick> {
        if self.numerator == 0 || self.denominator == 0 {
            return None;
        }
        let ticks_per_frame =
            i128::from(TICKS_PER_SECOND).checked_mul(i128::from(self.denominator))?;
        if ticks_per_frame % i128::from(self.numerator) != 0 {
            return None;
        }
        let frame_ticks =
            (ticks_per_frame / i128::from(self.numerator)).checked_mul(i128::from(frames))?;
        i64::try_from(frame_ticks).ok().map(Tick)
    }

    /// Returns the frame index at or before `time` (floor toward negative infinity).
    pub fn frame_at_or_before(self, time: Tick) -> Option<i64> {
        let numerator = i128::from(time.0).checked_mul(i128::from(self.numerator))?;
        let denominator = i128::from(TICKS_PER_SECOND).checked_mul(i128::from(self.denominator))?;
        i64::try_from(numerator.div_euclid(denominator)).ok()
    }
}

pub fn samples_to_ticks(samples: i64, sample_rate: u32) -> Option<Tick> {
    if sample_rate == 0 {
        return None;
    }
    if TICKS_PER_SECOND % i64::from(sample_rate) != 0 {
        return None;
    }
    let ticks = (i128::from(TICKS_PER_SECOND) / i128::from(sample_rate))
        .checked_mul(i128::from(samples))?;
    i64::try_from(ticks).ok().map(Tick)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_frame_rates_do_not_accumulate_drift() {
        let rate = FrameRate::FPS_2997;
        let frame_count = 30_000_i64 * 60 * 60;
        let time = rate.frames_to_ticks(frame_count).unwrap();
        assert_eq!(time.0, TICKS_PER_SECOND * 1_001 * 60 * 60);
        assert_eq!(rate.frame_at_or_before(time).unwrap(), frame_count);
    }

    #[test]
    fn common_video_and_audio_rates_have_exact_tick_boundaries() {
        assert_eq!(FrameRate::FPS_24.frame_ticks(), Some(10_584_000_000));
        assert_eq!(FrameRate::FPS_23976.frame_ticks(), Some(10_594_584_000));
        assert_eq!(
            samples_to_ticks(44_100, 44_100),
            Some(Tick(TICKS_PER_SECOND))
        );
    }

    #[test]
    fn rejects_invalid_rates_and_overflow() {
        assert!(FrameRate::new(0, 1).is_none());
        assert!(FrameRate::new(24, 0).is_none());
        assert!(samples_to_ticks(1, 0).is_none());
        assert!(FrameRate::FPS_30.frames_to_ticks(i64::MAX).is_none());
        assert!(FrameRate::new(13, 1).unwrap().frames_to_ticks(1).is_none());
        assert!(samples_to_ticks(1, 44_117).is_none());
    }
}
