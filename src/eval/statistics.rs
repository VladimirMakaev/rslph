//! Statistical computation for multi-trial evaluation results.
//!
//! Provides `StatSummary` for basic descriptive statistics and
//! `TrialStatistics` for aggregating eval metrics across trials.

use serde::Serialize;

/// Summary statistics for a set of values (EVAL-06).
#[derive(Debug, Clone, Serialize)]
pub struct StatSummary {
    /// Arithmetic mean of the values
    pub mean: f64,
    /// Sample variance (Bessel's correction: n-1 denominator)
    pub variance: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Number of values
    pub count: usize,
}

impl StatSummary {
    /// Compute summary statistics from a slice of values.
    ///
    /// - Empty slice: returns zeros for all fields
    /// - Single value: mean = value, variance = 0.0
    /// - Multiple values: uses Bessel's correction for sample variance
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                mean: 0.0,
                variance: 0.0,
                min: 0.0,
                max: 0.0,
                count: 0,
            };
        }

        let count = values.len();
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Sample variance with Bessel's correction (n-1)
        let variance = if count == 1 {
            0.0
        } else {
            let sum_squared_diff: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
            sum_squared_diff / (count - 1) as f64
        };

        Self {
            mean,
            variance,
            min,
            max,
            count,
        }
    }

    /// Standard deviation (square root of variance).
    pub fn std_dev(&self) -> f64 {
        self.variance.sqrt()
    }
}

/// Aggregated statistics across multiple trials (EVAL-07).
#[derive(Debug, Clone, Serialize)]
pub struct TrialStatistics {
    /// Pass rate statistics (0.0 to 1.0)
    pub pass_rate: StatSummary,
    /// Elapsed time in seconds
    pub elapsed_secs: StatSummary,
    /// Total input tokens consumed
    pub total_input_tokens: StatSummary,
    /// Total output tokens consumed
    pub total_output_tokens: StatSummary,
    /// Number of build iterations
    pub iterations: StatSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_summary_empty() {
        let summary = StatSummary::from_values(&[]);
        assert_eq!(summary.mean, 0.0);
        assert_eq!(summary.variance, 0.0);
        assert_eq!(summary.min, 0.0);
        assert_eq!(summary.max, 0.0);
        assert_eq!(summary.count, 0);
    }

    #[test]
    fn test_stat_summary_single() {
        let summary = StatSummary::from_values(&[42.0]);
        assert_eq!(summary.mean, 42.0);
        assert_eq!(summary.variance, 0.0);
        assert_eq!(summary.min, 42.0);
        assert_eq!(summary.max, 42.0);
        assert_eq!(summary.count, 1);
    }

    #[test]
    fn test_stat_summary_multiple() {
        // Values: [1.0, 2.0, 3.0, 4.0, 5.0]
        // Mean: 3.0
        // Sample variance: sum((x - mean)^2) / (n-1) = (4+1+0+1+4) / 4 = 2.5
        let summary = StatSummary::from_values(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(summary.mean, 3.0);
        assert_eq!(summary.variance, 2.5);
        assert_eq!(summary.min, 1.0);
        assert_eq!(summary.max, 5.0);
        assert_eq!(summary.count, 5);
    }

    #[test]
    fn test_stat_summary_std_dev() {
        let summary = StatSummary::from_values(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        // std_dev = sqrt(2.5) ≈ 1.5811
        assert!((summary.std_dev() - summary.variance.sqrt()).abs() < 1e-10);
        assert!((summary.std_dev() - 1.5811388300841898).abs() < 1e-10);
    }
}
