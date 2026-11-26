use bullet_lib::value::loader::ViriBinpackLoader;

pub fn data(path: &str) -> ViriBinpackLoader {
    let filter = viriformat::dataformat::Filter {
        // Tcheran's max eval value
        max_eval: 30000,

        // Defaults
        min_ply: 16,
        min_pieces: 4,

        filter_tactical: true,
        filter_check: true,
        filter_castling: false,

        max_eval_incorrectness: u32::MAX,

        random_fen_skipping: false,
        random_fen_skip_probability: 0.0,

        wdl_filtered: false,
        wdl_model_params_a: [6.871_558_62, -39.652_263_91, 90.684_603_52, 170.669_963_64],
        wdl_model_params_b: [-7.198_907_10, 56.139_471_85, -139.910_911_83, 182.810_074_27],
        material_min: 17,
        material_max: 78,
        mom_target: 58,
        wdl_heuristic_scale: 1.5,
    };

    let mut data_paths = std::fs::read_dir(path)
        .expect("Unable to find data dir")
        .map(|path| path.unwrap().path())
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    data_paths.sort();
    assert!(!data_paths.is_empty(), "No data files found");

    println!("Using data:");
    for path in &data_paths {
        println!("  - {path}");
    }

    ViriBinpackLoader::new_interleave_multiple(
        &data_paths.iter().map(String::as_str).collect::<Vec<_>>(),
        1024 * 8,
        8,
        filter,
    )
}
