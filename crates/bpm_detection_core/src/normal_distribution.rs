use std::ops::Index;

use chrono::Duration;
use statrs::distribution::{Continuous, Normal};

use crate::NormalDistributionParameters;

pub(crate) struct NormalDistribution {
    normal_distribution_data_points: Vec<f32>,
    pub normal_distribution_config: NormalDistributionParameters,
}

impl Index<Duration> for NormalDistribution {
    type Output = f32;

    fn index(&self, index: Duration) -> &Self::Output {
        &self.normal_distribution_data_points[self.index_for_duration(index)]
    }
}

impl NormalDistribution {
    fn index_for_duration(&self, duration: Duration) -> usize {
        ((duration.num_nanoseconds().unwrap() as f32 + self.normal_distribution_config.cutoff * 1_000_000.0)
            / (self.normal_distribution_config.resolution * 1_000_000.0))
            .round() as usize
    }

    fn duration_for_index(&self, index: usize) -> Duration {
        let nanos = (index as f32 * self.normal_distribution_config.resolution * 1_000_000.0)
            - self.normal_distribution_config.cutoff * 1_000_000.0;
        Duration::nanoseconds(nanos as i64)
    }

    fn size(&self) -> usize {
        ((2.0 * self.normal_distribution_config.cutoff / self.normal_distribution_config.resolution) + 1.0).ceil()
            as usize
    }

    pub fn new(normal_distribution_config: NormalDistributionParameters) -> Self {
        let mut this = Self { normal_distribution_data_points: vec![], normal_distribution_config };
        this.normal_distribution_data_points = this.make_normal_distribution();
        this
    }
}

impl Default for NormalDistribution {
    fn default() -> Self {
        let mut this = Self {
            normal_distribution_data_points: vec![],
            normal_distribution_config: NormalDistributionParameters::default(),
        };
        this.normal_distribution_data_points = this.make_normal_distribution();
        this
    }
}

impl NormalDistribution {
    pub(crate) fn make_normal_distribution(&mut self) -> Vec<f32> {
        let mean = 0.0;
        let std_dev = self.normal_distribution_config.std_dev;

        // Create the normal distribution
        let normal_dist = Normal::new(mean, std_dev).unwrap();

        // Data points for the line
        (0..self.size())
            .map(|x| {
                (normal_dist.pdf(self.duration_for_index(x).num_nanoseconds().unwrap() as f64 / 1_000_000.) as f32)
                    * self.normal_distribution_config.factor
            }) // Calculate probability density
            .collect::<Vec<_>>()
    }
}
