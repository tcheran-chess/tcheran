use bullet_lib::{
    game::{
        formats::bulletformat::ChessBoard,
        inputs::{ChessBucketsMirrored, SparseInputType},
        outputs::OutputBuckets,
    },
    wdl::WdlScheduler,
};
use bullet_trainer::model::{DenseInput, ModelInputs, ModelInputsMapper, SparseInput};

use crate::trainer::SCALE;

pub type InputTy = (((SparseInput, SparseInput), SparseInput), DenseInput<f32>);

pub fn make_inputs_mapper(
    params: (&ModelInputs<InputTy>, &ChessBucketsMirrored, impl OutputBuckets<ChessBoard>),
    wdl: impl WdlScheduler,
) -> ModelInputsMapper<ChessBoard> {
    let (inputs, psqt, output_buckets) = params;
    let psqt = *psqt;

    ModelInputsMapper::build(inputs, move |pos, step, (((stm_psqt, ntm_psqt), bucket), target)| {
        let mut psqt_count = 0;
        psqt.map_features(pos, |stm, ntm| {
            stm_psqt[psqt_count] = stm.try_into().unwrap();
            ntm_psqt[psqt_count] = ntm.try_into().unwrap();
            psqt_count += 1;
        });
        if psqt_count < psqt.max_active() {
            stm_psqt[psqt_count] = -1;
            ntm_psqt[psqt_count] = -1;
        }

        bucket[0] = i32::from(output_buckets.bucket(pos));

        let result = f32::from(pos.result) / 2.0;
        let score = 1.0 / (1.0 + (f32::from(-pos.score) / SCALE).exp());
        let lambda = wdl.blend(step.batch(), step.superbatch(), step.final_superbatch());
        assert!((0.0..=1.0).contains(&lambda), "WDL lambda must be in [0, 1]");
        target[0] = lambda * result + (1. - lambda) * score;
    })
}
