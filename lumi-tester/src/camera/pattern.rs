use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternSample {
    pub elapsed_ms: u64,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct BlinkPattern {
    pub state: String,
    pub count: u32,
    pub within_ms: u64,
    pub pulse_min_ms: u64,
    pub pulse_max_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlinkEvent {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternMatch {
    pub matched: bool,
    pub expected_state: String,
    pub expected_count: u32,
    pub observed_count: u32,
    pub events: Vec<BlinkEvent>,
    pub samples: Vec<PatternSample>,
}

pub fn detect_blinks(samples: &[PatternSample], pattern: &BlinkPattern) -> PatternMatch {
    let mut events = Vec::new();
    let mut active_start: Option<u64> = None;
    let state = pattern.state.trim();

    for sample in samples {
        let active = sample.state.trim().eq_ignore_ascii_case(state);
        match (active, active_start) {
            (true, None) => active_start = Some(sample.elapsed_ms),
            (false, Some(start)) => {
                push_event(&mut events, start, sample.elapsed_ms, pattern);
                active_start = None;
            }
            _ => {}
        }
    }

    if let (Some(start), Some(last)) = (active_start, samples.last()) {
        push_event(&mut events, start, last.elapsed_ms, pattern);
    }

    let observed_count = events.len() as u32;
    let need = pattern.count.max(1) as usize;
    let matched = events.windows(need).any(|window| {
        window
            .first()
            .zip(window.last())
            .map(|(first, last)| last.end_ms.saturating_sub(first.start_ms) <= pattern.within_ms)
            .unwrap_or(false)
    });

    PatternMatch {
        matched,
        expected_state: pattern.state.clone(),
        expected_count: pattern.count,
        observed_count,
        events,
        samples: samples.to_vec(),
    }
}

fn push_event(events: &mut Vec<BlinkEvent>, start: u64, end: u64, pattern: &BlinkPattern) {
    let duration_ms = end.saturating_sub(start);
    if duration_ms >= pattern.pulse_min_ms && duration_ms <= pattern.pulse_max_ms {
        events.push(BlinkEvent {
            start_ms: start,
            end_ms: end,
            duration_ms,
        });
    }
}

pub fn samples_window_ms(samples: &[PatternSample]) -> Duration {
    Duration::from_millis(samples.last().map(|s| s.elapsed_ms).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(elapsed_ms: u64, state: &str) -> PatternSample {
        PatternSample {
            elapsed_ms,
            state: state.to_string(),
        }
    }

    #[test]
    fn detects_three_pink_blinks_inside_window() {
        let samples = vec![
            s(0, "OFF"),
            s(40, "PINK"),
            s(120, "OFF"),
            s(180, "PINK"),
            s(250, "OFF"),
            s(310, "PINK"),
            s(390, "OFF"),
        ];
        let pattern = BlinkPattern {
            state: "PINK".to_string(),
            count: 3,
            within_ms: 800,
            pulse_min_ms: 20,
            pulse_max_ms: 200,
        };

        let result = detect_blinks(&samples, &pattern);
        assert!(result.matched);
        assert_eq!(result.observed_count, 3);
    }

    #[test]
    fn rejects_pulses_longer_than_max() {
        let samples = vec![s(0, "OFF"), s(10, "PINK"), s(400, "OFF")];
        let pattern = BlinkPattern {
            state: "PINK".to_string(),
            count: 1,
            within_ms: 800,
            pulse_min_ms: 20,
            pulse_max_ms: 200,
        };

        let result = detect_blinks(&samples, &pattern);
        assert!(!result.matched);
        assert_eq!(result.observed_count, 0);
    }

    #[test]
    fn ignores_early_noise_when_later_window_matches() {
        let samples = vec![
            s(0, "PINK"),
            s(30, "OFF"),
            s(1000, "PINK"),
            s(1080, "OFF"),
            s(1140, "PINK"),
            s(1220, "OFF"),
            s(1280, "PINK"),
            s(1360, "OFF"),
        ];
        let pattern = BlinkPattern {
            state: "PINK".to_string(),
            count: 3,
            within_ms: 500,
            pulse_min_ms: 20,
            pulse_max_ms: 200,
        };

        let result = detect_blinks(&samples, &pattern);
        assert!(result.matched);
        assert_eq!(result.observed_count, 4);
    }
}
