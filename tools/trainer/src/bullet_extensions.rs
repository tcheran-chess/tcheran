use bullet_lib::TrainingSteps;
use bullet_lib::nn::optimiser::RangerParams;
use std::ops::Range;

pub trait TrainingStepsImpl {
    fn default(n_superbatches: usize) -> Self;

    #[allow(unused, reason = "Only used when resuming training")]
    fn from_superbatch(n_superbatches: usize, start_superbatch: usize) -> Self;
}

impl TrainingStepsImpl for TrainingSteps {
    fn default(n_superbatches: usize) -> Self {
        TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            end_superbatch: n_superbatches,
        }
    }

    fn from_superbatch(n_superbatches: usize, start_superbatch: usize) -> Self {
        let mut steps = Self::default(n_superbatches);
        steps.start_superbatch = start_superbatch;
        steps
    }
}

pub trait OptimiserParamsExt {
    fn clipped(ranger: Range<f32>) -> Self;
}

impl OptimiserParamsExt for RangerParams {
    fn clipped(range: Range<f32>) -> Self {
        Self {
            min_weight: range.start,
            max_weight: range.end,
            ..Default::default()
        }
    }
}
