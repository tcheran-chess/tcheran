use bullet_lib::{
    game::{
        inputs::{ChessBucketsMirrored, get_num_buckets},
        outputs::MaterialCount,
    },
    nn::{
        InitSettings, Shape,
        optimiser::{AdamW, AdamWParams},
    },
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

const FEATURES: usize = 768;
const HIDDEN_LAYER: usize = 1024;

#[rustfmt::skip]
const BUCKET_LAYOUT: [usize; 32] = [
    0, 0, 1, 1,
    2, 2, 2, 2,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
];

const INPUT_BUCKETS: usize = get_num_buckets(&BUCKET_LAYOUT);
const OUTPUT_BUCKETS: usize = 8;

pub fn run() {
    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(ChessBucketsMirrored::new(BUCKET_LAYOUT))
        .output_buckets(MaterialCount::<OUTPUT_BUCKETS>)
        .save_format(&[
            // Merge in the factoriser weights
            SavedFormat::id("l0w")
                .transform(|store, weights| {
                    let factoriser = store.get("l0f").values.f32().repeat(INPUT_BUCKETS);
                    weights.into_iter().zip(factoriser).map(|(a, b)| a + b).collect()
                })
                .round()
                .quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").round().quantise::<i16>(QB).transpose(),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs, output_buckets| {
            // factoriser
            let l0f = builder.new_weights("l0f", Shape::new(HIDDEN_LAYER, FEATURES), InitSettings::Zeroed);
            let expanded_factoriser = l0f.repeat(INPUT_BUCKETS);

            // weights
            let mut l0 = builder.new_affine("l0", FEATURES * INPUT_BUCKETS, HIDDEN_LAYER);
            l0.weights = l0.weights + expanded_factoriser;

            let l1 = builder.new_affine("l1", 2 * HIDDEN_LAYER, OUTPUT_BUCKETS);

            // inference
            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden_layer = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden_layer).select(output_buckets)
        });

    // Accounting for factoriser weight magnitudes (as per Bullet example)
    let stricter_clipping = AdamWParams {
        max_weight: 0.99,
        min_weight: -0.99,
        ..Default::default()
    };
    trainer.optimiser.set_params_for_weight("l0w", stricter_clipping);
    trainer.optimiser.set_params_for_weight("l0f", stricter_clipping);

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

    let data = ViriBinpackLoader::new(
        "etc/data/data.viri",
        1024 * 20,
        4,
        viriformat::dataformat::Filter::default(),
    );

    let settings = LocalSettings {
        threads: 8,
        test_set: None,
        output_directory: "etc/checkpoints",
        batch_queue_size: 32,
    };

    trainer.run(&schedule1, &settings, &data);
    trainer.run(&schedule2, &settings, &data);
}
