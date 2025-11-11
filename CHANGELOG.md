# Changelog

## [Unreleased]

### Misc

* Update the minimum Rust version to 1.91
* Remove paste dev dependency
* Remove arrayvec dependency

## [7.0]

* Network #2, trained with bullet's `main` branch with the same parameters as before (48.88 +- 14.81)
* Network #3, retrained in the same way (and with the same data) as #2, but after re-shuffling the dataset (17.29 +- 8.50)
* Network #4, keeping the exact same architecture and training params as #3, but trained on ~450k FENs (89.86 +- 20.11)
* Network #5, with no changes to architecture or training, but trained using a fresh ~120k FEN dataset after fixing a crucial datagen bug re. accidentally inverted WDL scores (108.74 +- 20.12)
* Network #6, same as #5 but trained using increased (0.3) WDL proportion and reduced (0.001) learning rate (53.33 +- 15.20)
* Skip bad captures in quiescence search (43.07 +- 13.42)
* Use less time if the best move is stable (9.34 +- 5.77)
* Do tranposition table cutoff comparison on the mate-adjusted score (1.72 +- 3.81)

### Misc

* Fix TT mate scores being stored without root/position correction
* Hide transposition table implementation details in its module
* datagen now saves data in viriformat

## [6.0]

* Switched to NNUE-based evaluation with a (768->256)x2->1 net (223.69 +- 33.98)
* Update NNUE features in a single loop when moving a single piece (10.65 +- 6.23)
* Use `align(64)` for network weights (5.51 +- 4.06)
* Encode double pushes in `Move` instead of checking in `make_move` (7.46 +- 6.47 in regression)

### Misc

* Make the engine both a library and binary to allow splitting functionality out
* Split the texel tuner out of the main engine into its own tools/tuner project
* Add `tools/datagen` for data generation
* Add a full set of bench positions for 'bench' command
* When in a tablebase position, report the tablebase PV line
* Don't panic when SyzygyPath is set but is empty
* Rewrite the FEN and UCI parsers without using `nom` (and remove `nom` as a dependency)
* Remove the dependency on `colored`
* Remove the runtime dependency on `rand` by pre-computing Zobrist components
* Remove the Zobrist component for 'no en-passant target'
* Give better error messages if we encounter panics in search code
* Use Rust 1.85 and Edition 2024
* Add initial bench
* Removed accidental eval tracing from datagen, resulting in a 1.5x speedup
* Remove the texel tuning code entirely in preparation for NNUE
  This also resolves a bug where the engine was always tracing its evaluation
  when running with `cargo run` due to the `tuner` feature being automatically
  enabled, as the 'engine' bin is a sibling of the 'engine' lib and cannot
  control its enabled features.

## [5.1]

* Evaluate passed pawns (38.30 +- 13.27)
* Don't consider mobility for squares that are attacked by opponent pawns (14.90 +- 7.93)

### Misc

* Determine the evaluation coefficients in the main eval module, removing the need for a side-by-side impl in the tuner

## [5.0]

* Tune all evaluation parameters with https://github.com/GediminasMasaitis/texel-tuner (53.17 +- 16.76)
* Evaluate piece mobility (41.36 +- 13.82)
* Add a texel tuner in-repo and tune, resolving an issue where mobility scores were not computed correctly (28.53 +- 11.44)
* Evaluate king safety (24.27 +- 10.43)

### Misc

* Define PSTs directly using phased evaluations to prep for tuning
* Move all tunable parameters to a single file to prep for tuning
* Add the 'default' feature to enable functionality for local from-source builds without polluting release builds
* Add a CLI to the non-release build
* Hide fields from PersistentState inside SearchContext

## [4.1]

* Add tablebase probing in search based on 'fathom' (18.97 +- 8.35 (5-man))
* Use arrayvec for PrincipalVariation (15.52 +- 8.72)
* Fix an accidental fail hard in quiescence - fail soft when eval >= beta (12.54 +- 7.15)
* Add futility pruning (10.82 +- 6.41)
* Pack Move into u16 and store extra information, e.g. if the move is a capture (7.03 +- 4.89)
* Add tablebase support and follow tablebase lines (6.79 +- 4.59 (5-man vs none))
* Add an evaluation bonus for having a pair of bishops (4.35 +- 3.46)
* Add countermove history (2.66 +- 2.03)
* Use separate functions for scoring tacticals vs. quiets (2.64 +- 4.43)
* Enable LTO (2.09 +- 3.32)
* Pre-compute move flags (2.07 +- 4.01)
* LMR: Reduce in check, but reduce less than normal (1.51 +- 2.70)
* Use arrayvec for MoveList (1.05 +- 3.35)

### Misc

* Allow sending only 'go wtime' or 'go btime'
* Avoid constructing `Move` objects directly where possible, preferring to extract them from `MoveList`
* Hide the internal implementation details of `Move`
* Add a `ByPlayer` struct for easily working with values stored for both players

## [4.0]

* Add late move reductions (100.80 +- 22.10 Elo)
* Use SEE to order bad captures later in moves to try (6.69 +- 4.80 Elo)
* Do not allow TT cutoffs in PV nodes (~0 Elo)

### Misc

* Avoid locking up the UCI thread if the 'Hash' option is set during a search
* Add a new Github Action for building and publishing new releases

## [3.0]

* Use hard and soft time limits in our time management strategy (~28 Elo STC, ~43 Elo LTC)
* Store the board state as `[PieceOccupancy; Pieces]` and `[ColorOccupancy; Colors]` (~24 Elo)
* Collect the principal variation during search (~18 Elo)
* Use 3.3% of the remaining time as base rather than 5% (~9 Elo)
* Skip losing captures in quiescence (~8 Elo)
* Add aspiration windows (~5 Elo)
* Return `best_eval` in quiescence (~5 Elo)
* Fail soft on TT cuts (~2 Elo)
* Prefer TT nodes with a higher depth (~2 Elo)
* Don't do RFP or NMP in TT nodes (~0 Elo)

### Misc

* Clear `PersistentState` and `Control` on `ucinewgame` (fails SPRT at -9 Elo but is strictly more correct)
    * Allocate TT before the first search (gains +10 undoing the -9 from the time spent allocating after `ucinewgame` and before the first search)
* Add prettier search output if being used interactively
* Rename `MoveProvider` -> `MovePicker` for consistency with other engines
* Improved debug output when MovePicker perft tests fail
* Move search termination check and 'force stop' of search into `TimeStrategy`
* Fix OOM-kills due to briefly allocating two transposition tables during `ucinewgame`
* Remove `git-version` for setting UCI version dynamically during development

## [2.5]

* Fix not storing moves that caused beta cutoffs in the TT (~50 Elo)
* Pack midgame and endgame `PhasedEval` `i16`s into a single `i32` (~19 Elo)
* Do PVS by searching first move with the full window and the remainder with a zero-window (~4 Elo)
* Perform all TT updates in the same place in `negamax` (~1 Elo)

### Misc

* Use `UciMove` instead of `Move` in `uci`
* Encapsulate the history table in `HistoryTable`
* Encapsulate the killers table in `KillersTable`
* Remove `color-eyre` dependency

## [2.4]

* Expand move scoring range from 200000 to 1000000000 (~10 Elo)
* Refactor duplicate code in MoveProvider (~9 Elo)
* Score quiets after yielding killers and avoid scoring captures with killer scores (~7 Elo)
* Persist and decay history heuristic data (~6 Elo)
* Do null move pruning when static eval == beta (~5 Elo)
* Avoid yielding the prior best move from the transposition table twice (~2 Elo)
* Killer move fixes: bound history scores and don't allow dupes (~0 Elo)

### Misc

* Added a 'Threads' UCI option (which isn't used)
* Changed various Cargo options
    * Disable incremental compilation in release mode
    * Switch to panic=abort
    * Stop generating debug symbols
    * Set codegen-units=1
* Report 'uci name' as 'name version' instead of 'name (version)'

## [2.3]

* Implement lazy (staged) move generation (~25 Elo)
* Index the history heuristic array by player (~9 Elo)
* Fix a bug which overwrote killer moves with moves from another ply (~7 Elo)

### Misc

* Use atomics instead of a mutex for the shared 'stop' flag
* Allow making multiple moves with `d move [moves]`

## [2.2]

* Restore zobrist hash, incremental evaluation fields and castle rights from history when undoing move (~17 Elo)
* Fix throwing away old en passant target during null moves (~19 Elo)
* Use a dedicated `MoveList` struct instead of `Vec<Move>`
* Store castle rights as an array indexed by player
* Correct stored mate values in TT
* Make a panic move if there wasn't enough time to find a PV move during search

### Misc

* Split eval tapering into its own module
* Bundle midgame and endgame evals into a `PhasedEval` struct
* Fix taking up more memory than needed when the transposition table is resized repeatedly
* Remove the default 50ms move overhead and add a UCI option to configure it
* Always log crashes to a .crash.log file
* Check for time termination in the root
* Don't try reporting PV beyond actual depth searched

## [2.1]

* Use transposition table entries from the same depth (~101 Elo)
* Always extend when in check (~29 Elo)

### Misc

* Avoid double-counting 'root' quiescence nodes

## [2.0]

* Add null move pruning
* Add killer move ordering
* Sort moves via individual move scoring
* Sort moves incrementally
* Add reverse futility pruning
* Add history heuristic move ordering

### Misc

* Remove the ability to specify alternate strategies

## [1.1]

* Disable logging by default
* Use Rust 1.75
* Switch movegen to use orthogonal/diagonal pin approach from [this article](https://www.codeproject.com/Articles/5313417/Worlds-Fastest-Bitboard-Chess-Movegenerator)
* Use `.get_unchecked()` for all static array accesses (-1.18% perft(8) time)
* Store `Square` as a `u8` instead of a `Bitboard` internally
* Use an array for `PlayerPieces` (-6.37% search(9) time)
* Generate attackers for single squares instead of all attacks in movegen (+8.0 Elo)
* Optimise castle move generation
* Don't generate non-capture underpromotions in quiescence search
* Remove the `Ctx` struct from movegen (-3.13% perft(7) time)
* Reorganise everything into a single crate
* Re-enable incremental compilation for an unexplained performance boost in `sort_unstable_by`
* Consider bishops more valuable than knights for MVV-LVA

### Misc

* Add SAN parsing and formatting
* Add the 'Win At Chess' test suite
* Add Justfile commands for STC and LTC tests
* Use `u64` for node counts to prevent overflows with large perft results
* Remove support for `go searchmoves` and `go mate`
* Collapse castle detection for kingside/queenside into a single code path
* Various refactoring and simplification around `Bitboard` and `Square` abstractions
* Remove `EngineGame`
* Add a `wait` extension to allow piping `go` commands to the engine for benchmarking
* Make the halfmove clock and fullmove number optional in FEN parsing
* Add a way to easily jump to useful debugging positions (e.g. `d position kiwipete`)
* Add the ability to pass UCI commands to run as command line arguments

## [1.0]

Initial release with the following major features:

* Board
    * Bitboard board representation
    * Redundant mailbox representation for square lookups
    * Zobrist hashing

* Move generation
    * Fully legal move generation (~200 million NPS)
    * Fancy Magic bitboards

* Search
    * Negamax
    * Iterative deepening
    * Quiescence search
    * Principal Variation Search (PVS)
    * Check extensions
    * Transposition table

* Move ordering
    * Previous best move
    * Most Valuable Victim - Least Valuable Aggressor (MVV-LVA)

* Evaluation
    * Material difference
    * Midgame and endgame piece square tables
    * Tapered midgame vs. endgame evaluation
    * Incremental updates

[unreleased]: https://github.com/jgilchrist/chess-engine/compare/v7.0...HEAD
[7.0]: https://github.com/jgilchrist/chess-engine/compare/v6.0..v7.0
[6.0]: https://github.com/jgilchrist/chess-engine/compare/v5.1..v6.0
[5.1]: https://github.com/jgilchrist/chess-engine/compare/v5.0..v5.1
[5.0]: https://github.com/jgilchrist/chess-engine/compare/v4.1..v5.0
[4.1]: https://github.com/jgilchrist/chess-engine/compare/v4.0..v4.1
[4.0]: https://github.com/jgilchrist/chess-engine/compare/v3.0..v4.0
[3.0]: https://github.com/jgilchrist/chess-engine/compare/v2.5..v3.0
[2.5]: https://github.com/jgilchrist/chess-engine/compare/v2.4..v2.5
[2.4]: https://github.com/jgilchrist/chess-engine/compare/v2.3..v2.4
[2.3]: https://github.com/jgilchrist/chess-engine/compare/v2.2..v2.3
[2.2]: https://github.com/jgilchrist/chess-engine/compare/v2.1..v2.2
[2.1]: https://github.com/jgilchrist/chess-engine/compare/v2.0..v2.1
[2.0]: https://github.com/jgilchrist/chess-engine/compare/v1.1..v2.0
[1.1]: https://github.com/jgilchrist/chess-engine/compare/v1.0..v1.1
[1.0]: https://github.com/jgilchrist/chess-engine/releases/tag/v1.0
