#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SourceId(u64);

impl SourceId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChannelKey {
    pub(crate) source_id: SourceId,
    pub(crate) channel_path: String,
}

impl ChannelKey {
    pub(crate) fn new(source_id: SourceId, channel_path: impl Into<String>) -> Self {
        Self {
            source_id,
            channel_path: channel_path.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TimeTransform {
    pub(crate) offset_seconds: f64,
}

impl TimeTransform {
    pub(crate) fn to_display(self, raw_time: f64) -> f64 {
        raw_time + self.offset_seconds
    }

    pub(crate) fn to_raw(self, display_time: f64) -> f64 {
        display_time - self.offset_seconds
    }

    pub(crate) fn raw_range(self, display_range: (f64, f64)) -> (f64, f64) {
        (self.to_raw(display_range.0), self.to_raw(display_range.1))
    }

    pub(crate) fn display_range(self, raw_range: (f64, f64)) -> (f64, f64) {
        (self.to_display(raw_range.0), self.to_display(raw_range.1))
    }
}

impl Default for TimeTransform {
    fn default() -> Self {
        Self {
            offset_seconds: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn channel_key_identity_includes_source_and_path() {
        let source_1 = SourceId::new(1);
        let source_2 = SourceId::new(2);

        assert_eq!(
            ChannelKey::new(source_1, "iu"),
            ChannelKey::new(source_1, "iu")
        );
        assert_ne!(
            ChannelKey::new(source_1, "iu"),
            ChannelKey::new(source_2, "iu")
        );
        assert_ne!(
            ChannelKey::new(source_1, "iu"),
            ChannelKey::new(source_1, "iv")
        );
        assert_eq!(source_1.value(), 1);
        assert_eq!(source_2.value(), 2);
    }

    #[test]
    fn zero_offset_preserves_raw_and_display_time() {
        let transform = TimeTransform::default();
        assert_close(transform.to_display(1.25), 1.25);
        assert_close(transform.to_raw(1.25), 1.25);
        assert_eq!(transform.display_range((0.5, 1.5)), (0.5, 1.5));
        assert_eq!(transform.raw_range((0.5, 1.5)), (0.5, 1.5));
    }

    #[test]
    fn positive_and_negative_offsets_round_trip() {
        for offset in [2.5, -1.75] {
            let transform = TimeTransform {
                offset_seconds: offset,
            };
            let raw = 4.25;
            let display = transform.to_display(raw);
            assert_close(display, raw + offset);
            assert_close(transform.to_raw(display), raw);
        }
    }

    #[test]
    fn raw_and_display_ranges_round_trip() {
        let transform = TimeTransform {
            offset_seconds: -0.75,
        };
        let raw = (1.0, 3.5);
        let display = transform.display_range(raw);
        assert_close(display.0, 0.25);
        assert_close(display.1, 2.75);
        let round_trip = transform.raw_range(display);
        assert_close(round_trip.0, raw.0);
        assert_close(round_trip.1, raw.1);
    }
}
