use chrono::NaiveDate;

use crate::cli::Rating;
use crate::links::link_factor;

pub struct SrsInput {
    pub interval: u32,
    pub ease: f64,
    pub next_review: NaiveDate,
    pub today: NaiveDate,
    pub rating: Rating,
    pub link_count: usize,
    pub link_weight: f64,
    pub max_interval: u32,
}

pub struct SrsOutput {
    pub new_interval: u32,
    pub new_ease: f64,
    pub next_review: NaiveDate,
}

/// Core SRS calculation. All math in f64, single round at the end.
pub fn calculate(input: &SrsInput) -> SrsOutput {
    let delayed = (input.today - input.next_review).num_days().max(0) as f64;
    let interval = input.interval as f64;
    let ease = input.ease;

    let lf = link_factor(input.link_count);
    let effective_ease = ease * (1.0 + input.link_weight * lf);

    let (raw_interval, new_ease) = match input.rating {
        Rating::Hard => {
            let ni = (interval + delayed / 4.0) * 0.5;
            let ni = ni.max(1.0);
            let ne = (ease - 0.15_f64).max(1.3);
            (ni, ne)
        }
        Rating::Good => {
            let ni = (interval + delayed / 2.0) * effective_ease * 0.8;
            (ni, ease)
        }
        Rating::Easy => {
            let ni = (interval + delayed) * effective_ease;
            let ne = ease + 0.15;
            (ni, ne)
        }
    };

    // Clamp to max_interval, then round once
    let clamped = raw_interval.min(input.max_interval as f64);
    let new_interval = clamped.round() as u32;
    let new_interval = new_interval.max(1); // ensure at least 1

    let next_review = input.today + chrono::Duration::days(new_interval as i64);

    SrsOutput {
        new_interval,
        new_ease,
        next_review,
    }
}

/// Load balance: pick the date with fewest existing reviews in the fuzzing range.
/// Tie-break: earliest date.
pub fn load_balance(
    base_interval: u32,
    today: NaiveDate,
    existing_dates: &[NaiveDate],
) -> NaiveDate {
    let fuzz = match base_interval {
        1..=7 => 0_i32,
        8..=21 => 1,
        _ => {
            let pct = ((base_interval as f64) * 0.05).round() as i32;
            pct.min(3)
        }
    };

    if fuzz == 0 {
        return today + chrono::Duration::days(base_interval as i64);
    }

    let base_date = today + chrono::Duration::days(base_interval as i64);
    let start = base_date - chrono::Duration::days(fuzz as i64);
    let end = base_date + chrono::Duration::days(fuzz as i64);

    let mut best_date = start;
    let mut best_count = usize::MAX;

    let mut d = start;
    while d <= end {
        let count = existing_dates.iter().filter(|&&ed| ed == d).count();
        if count < best_count {
            best_count = count;
            best_date = d;
        }
        d += chrono::Duration::days(1);
    }

    best_date
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_good_no_delay_no_links() {
        let output = calculate(&SrsInput {
            interval: 1,
            ease: 2.5,
            next_review: date(2026, 2, 26),
            today: date(2026, 2, 26),
            rating: Rating::Good,
            link_count: 0,
            link_weight: 0.1,
            max_interval: 90,
        });
        // (1 + 0/2) * 2.5 * 0.8 = 2.0 → round = 2
        assert_eq!(output.new_interval, 2);
        assert_eq!(output.new_ease, 2.5);
        assert_eq!(output.next_review, date(2026, 2, 28));
    }

    #[test]
    fn test_hard_reduces_ease() {
        let output = calculate(&SrsInput {
            interval: 10,
            ease: 2.5,
            next_review: date(2026, 2, 20),
            today: date(2026, 2, 26),
            rating: Rating::Hard,
            link_count: 0,
            link_weight: 0.1,
            max_interval: 90,
        });
        // delayed = 6, (10 + 6/4) * 0.5 = (10 + 1.5) * 0.5 = 5.75 → 6
        assert_eq!(output.new_interval, 6);
        assert!((output.new_ease - 2.35).abs() < 0.001);
    }

    #[test]
    fn test_easy_increases_ease() {
        let output = calculate(&SrsInput {
            interval: 5,
            ease: 2.5,
            next_review: date(2026, 2, 26),
            today: date(2026, 2, 26),
            rating: Rating::Easy,
            link_count: 0,
            link_weight: 0.1,
            max_interval: 90,
        });
        // (5 + 0) * 2.5 = 12.5 → 13 (rounded)
        assert_eq!(output.new_interval, 13);
        assert!((output.new_ease - 2.65).abs() < 0.001);
    }

    #[test]
    fn test_max_interval_clamp() {
        let output = calculate(&SrsInput {
            interval: 80,
            ease: 2.5,
            next_review: date(2026, 2, 26),
            today: date(2026, 2, 26),
            rating: Rating::Easy,
            link_count: 0,
            link_weight: 0.1,
            max_interval: 90,
        });
        assert!(output.new_interval <= 90);
    }

    #[test]
    fn test_hard_min_ease() {
        let output = calculate(&SrsInput {
            interval: 1,
            ease: 1.3,
            next_review: date(2026, 2, 26),
            today: date(2026, 2, 26),
            rating: Rating::Hard,
            link_count: 0,
            link_weight: 0.1,
            max_interval: 90,
        });
        assert!((output.new_ease - 1.3).abs() < 0.001);
    }

    #[test]
    fn test_link_factor_affects_good() {
        let without = calculate(&SrsInput {
            interval: 10,
            ease: 2.5,
            next_review: date(2026, 2, 26),
            today: date(2026, 2, 26),
            rating: Rating::Good,
            link_count: 0,
            link_weight: 0.1,
            max_interval: 90,
        });
        let with_links = calculate(&SrsInput {
            interval: 10,
            ease: 2.5,
            next_review: date(2026, 2, 26),
            today: date(2026, 2, 26),
            rating: Rating::Good,
            link_count: 64,
            link_weight: 0.1,
            max_interval: 90,
        });
        assert!(with_links.new_interval > without.new_interval);
    }

    #[test]
    fn test_load_balance_no_fuzz_short_interval() {
        let result = load_balance(3, date(2026, 2, 26), &[]);
        assert_eq!(result, date(2026, 3, 1));
    }

    #[test]
    fn test_load_balance_picks_least_loaded() {
        let existing = vec![
            date(2026, 3, 7), // base-1
            date(2026, 3, 7),
            date(2026, 3, 8), // base
            date(2026, 3, 9), // base+1
        ];
        // interval=10, fuzz=±1, base=2026-03-08
        let result = load_balance(10, date(2026, 2, 26), &existing);
        // 3/7 has 2, 3/8 has 1, 3/9 has 1 → tie between 3/8 and 3/9 → earliest = 3/8
        assert_eq!(result, date(2026, 3, 8));
    }

    #[test]
    fn test_load_balance_tiebreak_earliest() {
        let existing: Vec<NaiveDate> = vec![];
        // interval=10, fuzz=±1, all have 0 reviews → pick earliest (base-1)
        let result = load_balance(10, date(2026, 2, 26), &existing);
        assert_eq!(result, date(2026, 3, 7)); // base(3/8) - 1 = 3/7
    }
}
