use super::Tablebase;
use crate::chess::Game;

fn test_csv(mut tables: Tablebase, path: &str) {
    tables
        .add_directory("src/engine/tablebases/syzygy/tables/chess")
        .expect("read directory");

    let mut reader = csv::Reader::from_path(path).expect("reader");

    for line in reader.records() {
        let record = line.expect("record");

        let fen: String = record
            .get(0)
            .expect("fen field")
            .parse()
            .expect("valid fen");

        let expected_wdl: i8 = record
            .get(1)
            .expect("wdl field")
            .parse()
            .expect("valid wdl");

        let expected_dtz: i32 = record
            .get(2)
            .expect("dtz field")
            .parse()
            .expect("valid dtz");

        let pos: Game = Game::from_frc_fen(&fen).expect("pos");

        println!("{fen} | wdl: {expected_wdl} | dtz: {expected_dtz}");

        match tables.probe_wdl_after_zeroing(&pos) {
            Ok(wdl) => assert_eq!(i8::from(wdl), expected_wdl),
            Err(err) => panic!("probe wdl: {err}"),
        }

        match tables.probe_dtz(&pos) {
            Ok(dtz) => assert_eq!(i32::from(dtz.ignore_rounding()), expected_dtz),
            Err(err) => panic!("probe dtz: {err}"),
        }
    }
}

#[cfg(any(unix, windows))]
#[test]
fn test_chess() {
    test_csv(Tablebase::new(), "src/engine/tablebases/syzygy/tests/chess.csv");
}

#[cfg(all(feature = "mmap", target_pointer_width = "64"))]
#[test]
fn test_chess_mmap() {
    // Safety: No modifications to table files and I/O errors please.
    // Fingers crossed.
    test_csv::<Chess>(
        unsafe { Tablebase::with_mmap_filesystem() },
        "src/engine/tablebases/syzygy/tests/chess.csv",
    );
}
