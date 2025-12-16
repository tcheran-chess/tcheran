use bullet_lib::{
    game::inputs::Chess768,
    nn::optimiser::AdamW,
    trainer::{
        save::SavedFormat,
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::{ValueTrainerBuilder, loader::ViriBinpackLoader},
};

const SCALE: i32 = 400;
const QA: i16 = 255;
const QB: i16 = 64;

fn main() {
    let hidden_size: usize = 1024;
    let wdl_proportion: f32 = 0.4;
    let superbatches: usize = 80;

    let mut trainer = ValueTrainerBuilder::default()
        .use_threads(8)
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(Chess768)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").round().quantise::<i16>(QB),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs| {
            // weights
            let l0 = builder.new_affine("l0", 768, hidden_size);
            let l1 = builder.new_affine("l1", 2 * hidden_size, 1);

            // inference
            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden_layer = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden_layer)
        });

    let schedule = TrainingSchedule {
        net_id: "tcheran".to_string(),
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            end_superbatch: superbatches,
        },
        wdl_scheduler: wdl::ConstantWDL {
            value: wdl_proportion,
        },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr: 0.001,
            final_lr: 0.001 * 0.3f32.powi(5),
            final_superbatch: superbatches,
        },
        save_rate: 10,
    };

    let settings = LocalSettings {
        threads: 8,
        test_set: None,
        output_directory: "etc/checkpoints",
        batch_queue_size: 32,
    };

    let data = ViriBinpackLoader::new(
        "etc/data/data.viri",
        1024 * 20,
        4,
        viriformat::dataformat::Filter::default(),
    );

    trainer.run(&schedule, &settings, &data);
}
