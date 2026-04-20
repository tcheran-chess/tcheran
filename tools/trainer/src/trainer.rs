use bullet_lib::{
    game::{inputs::ChessBucketsMirrored, outputs::MaterialCount},
    nn::optimiser::AdamW,
    trainer::{
        save::SavedFormat,
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::{ValueTrainerBuilder, loader::ViriBinpackLoader},
};

use crate::bullet_extensions::*;

const SCALE: f32 = 400.0;
const QA: i16 = 255;
const QB: i16 = 64;

const HIDDEN_LAYER: usize = 1024;
const OUTPUT_BUCKETS: usize = 8;

pub fn run() {
    let data = ViriBinpackLoader::new(
        "etc/data/data.viri",
        1024 * 20,
        4,
        viriformat::dataformat::Filter::default(),
    );

    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(ChessBucketsMirrored::default())
        .output_buckets(MaterialCount::<OUTPUT_BUCKETS>)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w")
                .round()
                .quantise::<i16>(QB)
                .transpose(),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs, output_buckets| {
            // weights
            let l0 = builder.new_affine("l0", 768, HIDDEN_LAYER);
            let l1 = builder.new_affine("l1", 2 * HIDDEN_LAYER, OUTPUT_BUCKETS);

            // inference
            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden_layer = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden_layer).select(output_buckets)
        });

    let settings = LocalSettings {
        threads: 8,
        test_set: None,
        output_directory: "etc/checkpoints",
        batch_queue_size: 32,
    };

    // Schedule 1
    let schedule1_superbatches: usize = 80;
    let schedule1 = TrainingSchedule {
        net_id: "tcheran".to_string(),
        eval_scale: SCALE,
        steps: TrainingSteps::default(schedule1_superbatches),
        wdl_scheduler: wdl::ConstantWDL { value: 0.4 },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr: 1e-3,
            final_lr: 2.7e-5,
            final_superbatch: schedule1_superbatches,
        },
        save_rate: 10,
    };

    trainer.run(&schedule1, &settings, &data);

    // Schedule 2
    let schedule2_superbatches: usize = 40;
    let schedule2 = TrainingSchedule {
        net_id: "tcheran-ft".to_string(),
        eval_scale: SCALE,
        steps: TrainingSteps::default(schedule2_superbatches),
        wdl_scheduler: wdl::ConstantWDL { value: 0.6 },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr: 4.5e-5,
            final_lr: 4.05e-6,
            final_superbatch: schedule2_superbatches,
        },
        save_rate: 10,
    };

    trainer.run(&schedule2, &settings, &data);
}
