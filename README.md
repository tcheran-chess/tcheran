<div align="center">

  <img alt="Logo" src=".github/logo.svg" width="200"></img>
  <h1>Tcheran</h1>

</div>

Tcheran is a UCI chess (and fischer random chess) engine developed by
[@jgilchrist](https://github.com/jgilchrist) and written in Rust.

For searching, it uses a standard alpha-beta search with many enhancements
(e.g. additional pruning conditions, heuristics and history mechanisms to
improve move ordering, etc.).

For evaluation, it uses an efficiently-updatable neural network (NNUE). Its
current architecture is `(768x8hm->1024)x2->(16->32->1)x8` (a multi-layer,
horizontally-mirrored network with 1024, 16 and 32 node hidden layers, and eight output
buckets). The network is trained exclusively using self-play games from earlier
versions of Tcheran, and [that data is available online][training-data]. More
details, including a detailed network history are available in
[NET.md](./NET.md).

[training-data]: https://huggingface.co/datasets/jgilchrist/tcheran/tree/main

## Ratings

| Version | [COPE Rapid][cope-rapid] | [COPE Bullet][cope-bullet] | [COPE FRC][cope-frc] | [COPE 8-CPU][cope-8cpu]  | [IPMan R9][ipman-r9] | [CCRL 40/15][ccrl-ltc] | [CCRL Blitz][ccrl-blitz] | [CCRL FRC][ccrl-frc] |
| ------- | ------------------------ | -------------------------- | -------------------- | ------------------------ | -------------------- | ---------------------- | ------------------------ | -------------------- |
| v14.0   | 3660 (#17)               | 3663 (#18)                 |                      | 3666 (#16)               | 3494 (#30)           |                        | 3695 (#30)               | 3970 (#23)           |
| v13.0   | 3626                     | 3621                       | 3647                 | 3610                     |                      | 3563                   | 3684                     | 3945                 |
| v12.0   |                          |                            |                      |                          |                      | 3534                   | 3651                     | 3806                 |
| v11.0   |                          |                            |                      |                          |                      | 3514                   | 3635                     |                      |
| v10.0   |                          |                            |                      |                          |                      | 3459                   | 3568                     |                      |
| v9.0    |                          |                            |                      |                          |                      | 3366                   | 3415                     |                      |
| v8.0    |                          |                            |                      |                          |                      | 3323                   |                          |                      |
| v7.0    |                          |                            |                      |                          |                      | 3174                   | 3231                     |                      |
| v6.0    |                          |                            |                      |                          |                      | 2917                   | 2975                     |                      |
| v5.1    |                          |                            |                      |                          |                      | 2698                   |                          |                      |
| v5.0    |                          |                            |                      |                          |                      | 2642                   | 2728                     |                      |
| v4.1    |                          |                            |                      |                          |                      |                        |                          |                      |
| v4.0    |                          |                            |                      |                          |                      | 2519                   | 2546                     |                      |
| v3.0    |                          |                            |                      |                          |                      | 2427                   | 2481                     |                      |
| v2.5    |                          |                            |                      |                          |                      | 2370                   |                          |                      |
| v2.4    |                          |                            |                      |                          |                      |                        | 2305                     |                      |
| v2.3    |                          |                            |                      |                          |                      |                        | 2328                     |                      |
| v2.2    |                          |                            |                      |                          |                      |                        | 2264                     |                      |
| v2.1    |                          |                            |                      |                          |                      | 2277                   | 2227                     |                      |
| v2.0    |                          |                            |                      |                          |                      |                        |                          |                      |
| v1.1    |                          |                            |                      |                          |                      |                        |                          |                      |
| v1.0    |                          |                            |                      |                          |                      |                        | 1868                     |                      |

[cope-rapid]: https://cope-chess.live/ratings?rating_list_id=6
[cope-bullet]: https://cope-chess.live/ratings?rating_list_id=5
[cope-frc]: https://cope-chess.live/ratings?rating_list_id=8
[cope-8cpu]: https://cope-chess.live/ratings?rating_list_id=7
[ipman-r9]: https://ipmanchess.yolasite.com/r9-7945hx.php
[ccrl-ltc]: https://computerchess.org.uk/ccrl/4040/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on&num_best_in_class=1&print=Rating+list&profile_step=50&profile_numbers=1&print=Results+table&print=LOS+table&table_size=100&ct_from_elo=0&ct_to_elo=10000&match_length=30&cross_tables_for_best_versions_only=1&sort_tables=by+rating&diag=0&reference_list=None&recalibrate=no
[ccrl-blitz]: https://computerchess.org.uk/ccrl/404/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on&num_best_in_class=1&print=Rating+list&profile_step=50&profile_numbers=1&print=Results+table&print=LOS+table&table_size=100&ct_from_elo=0&ct_to_elo=10000&match_length=30&cross_tables_for_best_versions_only=1&sort_tables=by+rating&diag=0&reference_list=None&recalibrate=no
[ccrl-frc]: https://computerchess.org.uk/404FRC/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on&num_best_in_class=1&table_size=100&ct_from_elo=0&ct_to_elo=10000&match_length=30&cross_tables_for_best_versions_only=1&sort_tables=by+rating&diag=0&recalibrate=no

It can also be found on Lichess as [`jpg-bot`](https://lichess.org/@/jpg-bot) where its ratings are:

[![lichess-rapid](https://lichess-shield.vercel.app/api?username=jpg-bot&format=bullet)](https://lichess.org/@/jpg-bot/perf/bullet)
[![lichess-rapid](https://lichess-shield.vercel.app/api?username=jpg-bot&format=blitz)](https://lichess.org/@/jpg-bot/perf/blitz)
[![lichess-rapid](https://lichess-shield.vercel.app/api?username=jpg-bot&format=rapid)](https://lichess.org/@/jpg-bot/perf/rapid)

## Usage

To run Tcheran, you can download the latest release from its [releases page][releases].

If you would instead like to build it from source:

* Ensure you have the pre-requisites installed:
    * Rust (the minimum supported version is listed in [Cargo.toml](./Cargo.toml))
    * A C compiler to build [Fathom][fathom] for tablebase probing support
* Run `cargo build --release` - this will download the latest network and build the engine.
* The engine binary will be available as `target/release/tcheran`.

[releases]: https://github.com/tcheran-chess/tcheran/releases

## Thanks

A huge thanks to the following people and tools:

* [Bullet][bullet], which is used for training Tcheran's neural networks
* [Fathom][fathom], which is used for Syzygy tablebase probing
* [OpenBench][openbench], which is used for testing changes to the engine
* [Stockfish's WDL model][stockfish-wdl], which is used for normalizing Tcheran's evaluation output
* @JonathanHallstrom for training some networks for Tcheran
* The testers at [CCRL][ccrl] (and elsewhere), who have invested time and hardware resource, and provided consistent motivation to improve

[bullet]: https://github.com/jw1912/bullet
[fathom]: https://github.com/jdart1/Fathom
[openbench]: https://github.com/AndyGrant/OpenBench
[stockfish-wdl]: https://github.com/official-stockfish/WDL_model
[ccrl]: https://www.computerchess.org.uk/ccrl/
