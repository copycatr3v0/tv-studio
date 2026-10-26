//! Summaries and before/after comparison that refuse to over-claim: with too few samples or
//! overlapping spreads the verdict is "no clear change".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats { pub n: usize, pub median: f64, pub p25: f64, pub p75: f64, pub p95: f64, pub min: f64, pub max: f64 }

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.len() == 1 { return sorted[0]; }
    let pos = q * (sorted.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

pub fn stats(values: &[f64]) -> Option<Stats> {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() { return None; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(Stats { n: v.len(), median: quantile(&v, 0.5), p25: quantile(&v, 0.25), p75: quantile(&v, 0.75), p95: quantile(&v, 0.95), min: v[0], max: v[v.len() - 1] })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict { Better, Worse, NoClearChange, NotEnoughData }

pub const MIN_SAMPLES: usize = 5;

/// Lower is better for every metric this app measures (ms, % CPU, MB).
/// "Better"/"Worse" only when both have enough samples and the interquartile ranges don't overlap.
pub fn compare(before: Option<&Stats>, after: Option<&Stats>) -> Verdict {
    let (Some(b), Some(a)) = (before, after) else { return Verdict::NotEnoughData };
    if b.n < MIN_SAMPLES || a.n < MIN_SAMPLES { return Verdict::NotEnoughData; }
    if a.p75 < b.p25 { Verdict::Better } else if a.p25 > b.p75 { Verdict::Worse } else { Verdict::NoClearChange }
}

/// Single-value metrics (one idle-CPU or PSS reading per run) can't show spread: report the
/// difference but never call it better or worse unless it exceeds `noise`.
pub fn compare_single(before: Option<f64>, after: Option<f64>, noise: f64) -> Verdict {
    match (before, after) {
        (Some(b), Some(a)) if (a - b).abs() <= noise => Verdict::NoClearChange,
        (Some(b), Some(a)) => if a < b { Verdict::Better } else { Verdict::Worse },
        _ => Verdict::NotEnoughData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles() {
        let s = stats(&[5.0, 1.0, 3.0, 2.0, 4.0]).unwrap();
        assert_eq!((s.n, s.median, s.min, s.max), (5, 3.0, 1.0, 5.0));
        assert_eq!((s.p25, s.p75), (2.0, 4.0));
        assert!(stats(&[]).is_none());
        assert!(stats(&[f64::NAN]).is_none());
    }

    #[test]
    fn verdicts_need_separation_and_samples() {
        let slow = stats(&[30.0, 32.0, 31.0, 33.0, 29.0, 34.0]).unwrap();
        let fast = stats(&[18.0, 19.0, 17.0, 20.0, 18.5, 19.5]).unwrap();
        let mixed = stats(&[25.0, 31.0, 18.0, 33.0, 20.0, 30.0]).unwrap();
        assert_eq!(compare(Some(&slow), Some(&fast)), Verdict::Better);
        assert_eq!(compare(Some(&fast), Some(&slow)), Verdict::Worse);
        assert_eq!(compare(Some(&slow), Some(&mixed)), Verdict::NoClearChange);
        let few = stats(&[10.0, 11.0]).unwrap();
        assert_eq!(compare(Some(&slow), Some(&few)), Verdict::NotEnoughData);
        assert_eq!(compare(None, Some(&fast)), Verdict::NotEnoughData);
        assert_eq!(compare_single(Some(5.0), Some(4.6), 1.0), Verdict::NoClearChange);
        assert_eq!(compare_single(Some(5.0), Some(2.0), 1.0), Verdict::Better);
    }
}
