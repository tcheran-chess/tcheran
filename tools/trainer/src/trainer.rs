use bullet_lib::{
    game::{
        inputs::{ChessBucketsMirrored, get_num_buckets},
        outputs::MaterialCount,
    },
    nn::{
        InitSettings, Shape,
        optimiser::{Ranger, RangerOptimiser, RangerParams},
    },
    trainer::{
        save::SavedFormat,
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::{ValueTrainer, ValueTrainerBuilder, loader::ViriBinpackLoader},
};

use crate::bullet_extensions::*;

type Optimiser = Ranger;
type OptimiserT = RangerOptimiser;
type Params = RangerParams;

pub const SCALE: f32 = 400.0;

pub const Q0: i16 = 255;
pub const Q1: i16 = 128;
pub const Q: i32 = 64;

pub const FEATURES: usize = 768;
pub const L1: usize = 1024;
pub const L2: usize = 16;
pub const L3: usize = 32;

#[rustfmt::skip]
const BUCKET_SCHEME: [usize; 32] = [
    0, 1, 2, 3,
    4, 4, 5, 5,
    6, 6, 6, 6,
    6, 6, 6, 6,
    7, 7, 7, 7,
    7, 7, 7, 7,
    7, 7, 7, 7,
    7, 7, 7, 7,
];

pub const INPUT_BUCKETS: usize = get_num_buckets(&BUCKET_SCHEME);
pub const OUTPUT_BUCKETS: usize = 8;

const L1_SHIFT: usize = 8;
const L1_SHIFT_SCALE: f32 = Q0 as f32 / ((1 << L1_SHIFT) as f32);
const I8_RANGE: f32 = i8::MAX as f32 / (Q1 as f32);
const L1_RANGE: f32 = I8_RANGE * L1_SHIFT_SCALE * L1_SHIFT_SCALE;

pub fn trainer() -> ValueTrainer<OptimiserT, ChessBucketsMirrored, MaterialCount<OUTPUT_BUCKETS>> {
    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(Optimiser::default())
        .inputs(ChessBucketsMirrored::new(BUCKET_SCHEME))
        .output_buckets(MaterialCount::<OUTPUT_BUCKETS>)
        .save_format(&[
            // Merge in the factoriser weights
            SavedFormat::id("l0w")
                .transform(|store, weights| {
                    let factoriser = store.get("l0f").values.f32().repeat(INPUT_BUCKETS);
                    weights.into_iter().zip(factoriser).map(|(a, b)| a + b).collect()
                })
                .round()
                .quantise::<i16>(Q0),
            SavedFormat::id("l0b").round().quantise::<i16>(Q0),
            SavedFormat::id("l1w")
                .transform(|_, mut weights| {
                    for i in weights.iter_mut() {
                        *i /= L1_SHIFT_SCALE * L1_SHIFT_SCALE;
                    }
                    weights
                })
                .round()
                .quantise::<i8>(Q1),
            SavedFormat::id("l1b").round().quantise::<i32>(Q * (1 << L1_SHIFT)),
            SavedFormat::id("l2w").round().quantise::<i32>(Q),
            SavedFormat::id("l2b").round().quantise::<i32>(Q.pow(3)),
            SavedFormat::id("l3w").round().quantise::<i32>(Q),
            SavedFormat::id("l3b").round().quantise::<i32>(Q.pow(4)),
        ])
        .build_custom(|builder, (stm_inputs, ntm_inputs, output_buckets), target| {
            // features & factoriser
            let l0f = builder.new_weights("l0f", Shape::new(L1, FEATURES), InitSettings::Zeroed);
            let mut l0 = builder.new_affine("l0", FEATURES * INPUT_BUCKETS, L1);
            l0.init_with_effective_input_size(20000);
            l0.weights = l0.weights + l0f.repeat(INPUT_BUCKETS);

            // weights
            let l1 = builder.new_affine("l1", L1, OUTPUT_BUCKETS * L2);
            let l2 = builder.new_affine("l2", L2, OUTPUT_BUCKETS * L3);
            let l3 = builder.new_affine("l3", L3, OUTPUT_BUCKETS);

            // inference
            let ft = |input, start, end| l0.slice(start, end).forward(input).crelu();
            let stm_hidden = ft(stm_inputs, 0, L1 / 2) * ft(stm_inputs, L1 / 2, L1);
            let ntm_hidden = ft(ntm_inputs, 0, L1 / 2) * ft(ntm_inputs, L1 / 2, L1);

            let h1 = stm_hidden.concat(ntm_hidden);
            let h2 = l1.forward(h1).select(output_buckets).screlu();
            let h3 = l2.forward(h2).select(output_buckets).crelu();
            let output = l3.forward(h3).select(output_buckets);

            // loss
            let main_loss = output.sigmoid().squared_error(target);
            let loss = main_loss;

            (output, loss)
        });

    // Accounting for factoriser weight magnitudes (as per Bullet example)
    let l0_clipping = Params::clipped(-0.99..0.99);
    trainer.optimiser.set_params_for_weight("l0w", l0_clipping);
    trainer.optimiser.set_params_for_weight("l0f", l0_clipping);

    let l1_clipping = Params::clipped(-L1_RANGE..L1_RANGE);
    trainer.optimiser.set_params_for_weight("l1w", l1_clipping);

    trainer
}

pub fn run(net_name: &str) {
    let mut trainer = trainer();

    // Schedule 1
    let schedule1_superbatches: usize = 400;
    let schedule1 = TrainingSchedule {
        net_id: format!("{net_name}-s1"),
        eval_scale: SCALE,
        steps: TrainingSteps::default(schedule1_superbatches),
        wdl_scheduler: wdl::Warmup {
            warmup_batches: 100,
            inner: wdl::LinearWDL { start: 0.2, end: 0.6 },
        },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr: 0.001,
            final_lr: 0.0000081,
            final_superbatch: schedule1_superbatches,
        },
        save_rate: 10,
    };

    // Schedule 2
    let schedule2_superbatches: usize = 100;
    let schedule2 = TrainingSchedule {
        net_id: format!("{net_name}-s2"),
        eval_scale: SCALE,
        steps: TrainingSteps::default(schedule2_superbatches),
        wdl_scheduler: wdl::ConstantWDL { value: 0.75 },
        lr_scheduler: lr::ConstantLR { value: 0.00000081 },
        save_rate: 10,
    };

    let data = ViriBinpackLoader::new(
        "etc/data/data.viri",
        1024 * 8,
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
