# Changelog

## [Unreleased]

* Use the sum of all conthist scores as the base for updates (9.09 +- 5.17)
* Added 1-ply continuation correction history (8.23 +- 5.04)
* Tweak LMR params: tacticals less, quiets more (7.42 +- 4.53)
* The TT score is now used for RFP if conditions allow (4.11 +- 3.09)
* Add late move pruning in quiescence (3.51 +- 2.75)
* Give a higher history bonus for moves that failed high but where the static eval was below alpha (2.66 +- 2.14)
* Add 4-ply continuation history (2.16 +- 1.73)
* LMR is now done later in the root node (0.50 +- 1.83)

#### Non-regression

* Saturating behaviour in LMR is no longer used (2.29 +- 3.43)
* Post-LMR re-search no longer happens if we didn't actually reduce due to clamping (-0.36 +- 2.51)
* Simplify killer moves (-0.81 +- 2.01)

#### Misc

* We now prefetch transposition table entries on aarch64 (13.79 +- 6.45 VSTC)
* All of the individual search files under search/ are merged into a search.rs
* SAN checkmate moves are now correctly suffixed with #
* Pretty info PVs are now truncated for readability, and checks, promotions and checkmates are colored

## [13.0]

* Network #16, adding 4 input buckets and trained with an additional ~2.1B positions (19.04 +- 7.72)
* Network #21, a multi-layer net (16.47 +- 7.08)
* Network #17, adding ~185M DFRC positions (91.09 +- 18.98 DFRC)
* Network #18, with a new much longer training schedule about half way between my existing one and Hobbes' (14.19 +- 6.56)
* Network #23, with dual activation for L2 (7.21 +- 4.45)
* Cut off if the singular beta beats beta (6.97 +- 4.40)
* Added TT clusters (6.28 +- 4.04)
* SPSA tune - 2682 iters at 20+0.20 (5.58 +- 3.78)
* Score quiet moves that give check higher for picking (4.97 +- 3.54)
* Preparation for making time management tunable (4.68 +- 4.31)
* Game::is_direct_check is now used in LMP (4.57 +- 3.35)
* Network #22, switching back to training multi-layer with Ranger (4.47 +- 3.26)
* Re-introduce futility pruning and make it depth dependent (4.38 +- 3.25)
* Add quiescence futility pruning (4.32 +- 3.18)
* Network #20, trained with Ranger instead of AdamW (3.54 +- 2.80)
* Reduce less for 'ttpv' nodes (3.12 +- 2.48)
* Fixed TT relative age calculation (2.91 +- 2.35)
* Network #19, upping the number of input buckets from 4 to 8 (2.95 +- 2.37)
* The reverse futility pruning margin is adjusted if we are improving (2.68 +- 2.34)
* Do qsearch evasion check after making move (2.37 +- 2.42)
* Legal killer moves are now yielded before generating quiet moves for a minor speedup
* A single stop flag is now used for all threads (5.69 +- 4.73 reg)
* Network #24, trained with Tcheran's actual max eval (30000) and tested non-reg

#### Misc

* Dev versions of the engine now contain the git SHA it was built from
* The TT move is now fully checked for legality, avoiding rare crashes
* Underpromotion captures are now scored properly
* Fixed empty TT entries being considered to have 'exact' bounds
* Added the ability for genfens to generate DFRC starting positions
* The 'Minimal' UCI option is now respected
* Threading code was rewritten to use the stdlib, removing the atomic-wait dependency
* The 'Move Overhead' UCI option is now 'MoveOverhead'
* The transposition table is now cleared using multiple threads
* bmi2 instructions are no longer disabled in some release builds
* A linux-aarch64 build is now available
* The WDL model has been updated

## [12.0]

* SPSA tune - 938 iters at 20.0+0.20 (24.36 +- 8.47)
* Network #15, same as 14 with an additional ~416M 20ksn positions (19.22 +- 7.60)
* SPSA tune - 3580 iters at 20.0+0.20 (16.55 +- 7.10)
* SPSA tune (1560 iters at 20.0+0.20) (7.87 +- 4.67)
* Exclude pinned pieces in SEE (7.58 +- 4.67)
* SPSA tune - 1077 iters at 20.0+0.20 (4.06 +- 3.05)
* The en-passant square is no longer set when there are no legal en-passant moves (4.57 +- 4.38)
* Use lmr_depth for moveloop pruning conditions (3.81 +- 2.95)
* Added quiet history (1.35 +- 2.90)
* Implement a proper thread pool
* Add mate distance pruning
* Stop returning mate scores from NMP
* The default move overhead is now set at 20ms

#### Misc

* Tablebase scores are now reported in tablebase positions instead of incorrect mate scores

## [11.0]

### Note

**11.0 was re-released with a fix for a critical bug that caused the engine to
crash whenever it entered a tablebase position at the root.**

* Network #11, trained with an entirely new 5k soft node dataset (23.37 +- 8.61)
* Use capture history as the SEE threshold during move picking (13.97 +- 6.76)
* Time management tweaks - base hard time on the total time and allow using more of the total time (22.35 +- 8.36 STC, 12.20 +- 5.91 LTC)
* Scale soft time based on the fraction of nodes used to search the bset move (10.89 +- 5.82 STC, 16.01 +- 6.94 LTC)
* Switch to handwritten SIMD for NNUE output layer (10.56 +- 5.63)
* Shrink default aspiration window from 25 to 20 (10.03 +- 5.56)
* Network #12, adding 8 output buckets and using the same data as #11 (9.89 +- 5.56)
* Do more LMR in cut nodes (9.87 +- 5.55)
* Use Lizard's SCReLU formulation (9.18 +- 5.16)
* Do NMP in cut nodes only (8.61 +- 5.05)
* Reduce more in LMR if the upcoming ply has failed high (7.80 +- 4.80)
* Add razoring (7.73 +- 4.76)
* Network #13, trained using the same arch and data as #12 but with horizontal mirroring (7.49 +- 4.59)
* Add threats to capture history (6.37 +- 4.12)
* Only set best_move if we raised alpha (6.13 +- 4.10)
* Clear killer moves for upcoming plies (5.77 +- 3.92)
* Generate quiets to evade check in quiescence (5.16 +- 3.69)
* Network #14, using the same data and architecture as net 13 but trained for an additional 40 superbatches at 0.6 WDL (5.00 +- 3.60)
* Set SearchStackEntry::mv in quiescence (4.77 +- 3.47)
* Reduce negative extension when tt_score >= beta (4.56 +- 3.38)
* Use capture history in SEE pruning threshold (4.01 +- 3.05)
* Negative extension when tt_score >= beta (3.66 +- 2.85)
* Shrink aspiration windows further, from 20 to 15 (3.40 +- 2.70)
* Return average of eval and beta for quiescence fail-highs (2.86 +- 2.29)
* Use threefold repetitions (2.49 +- 3.65)

### Misc

* Tcheran now supports Fischer Random Chess (FRC)
* We now always report a line immediately before printing the best move
* The 'genfens' UCI command is now implemented to support OpenBench datagen
* tools/data now has a command to convert from .pgn to viriformat
* The max thread count is now always 1024 rather than being detected
* The minimum required Rust version is now 1.93
* The engine no longer crashes when passed UCI options is doesn't recognise.
* UCI_ShowWDL is now a supported option.

## [10.0]

* Start using the transposition table in quiescence (40.16 +- 11.74)
* Add singular extensions (19.91 +- 8.22 STC, 34.02 +- 10.72 LTC)
* Add pawn eval correction history (21.76 +- 8.62)
* Add major and minor piece eval correction history (18.86 +- 7.99)
* Aspiration - reduce on fail high, base window on new eval, adjust beta when widening down (14.95 +- 6.96)
* Add non-pawn eval correction history (12.69 +- 6.30)
* Add internal iterative reductions (IIR) (9.66 +- 5.53)
* Use threats in quiet move history (9.28 +- 5.28 at LTC with threat calculation, 18.34 +- 7.98 STC excluding threat calculation)
* Add late move pruning (9.63 +- 5.59)
* Add threat eval correction history (9.30 +- 5.31)
* Add double extensions (9.28 +- 5.26)
* Revert net scale change with a view to using WDL normalisation instead (9.08 +- 5.12)
* Avoid re-allocating thread data before each search (8.95 +- 5.20 with 2 threads vs 2 threads, +~20 Elo in 2v1 thread odds)
* Add 1-ply continuation history (8.88 +- 5.20)
* Prefetch transposition table entries on X86 (8.65 +- 5.03)
* Add multicut in singular search (7.99 +- 4.84)
* Use beta + (eval - beta) / 3 formula for RFP (7.22 +- 4.67)
* Start singular extensions earlier (6.23 +- 4.08 LTC, 21.11 +- 8.32 STC)
* Scale evaluation by remaining material (6.02 +- 4.02)
* Introduce the improving heuristic and do less LMP if not improving (5.75 +- 4.00)
* LMR: Reduce non-PV nodes more (3.91 +- 3.03)
* Look back four plies for improving heuristic (1.91 +- 1.86)
* Use a faster way of calculating the transposition table index (2.87 +- 3.14)
* Use SEE values for scoring tacticals (1.59 +- 3.27)
* Simplify countermoves and move quiets before bad captures (1.35 +- 3.17)
* Add 2-ply continuation history (0.05 +- 2.20)

### Misc

* All reported evaluations are now normalised using a model derived from Stockfish's WDL\_model tool
* Add scaffolding for SPSA
* Switch all tunable params to be defined using macros

## [9.0]

* Implement support for Threads via Lazy SMP (39.62 +- 11.20 2v1, 105.71 +- 15.83 4v1)
* Do NNUE updates lazily (33.13 +- 10.96)
* Prevent accidental accumulator copies (12.95 +- 6.59)
* Network #10, upping the size to 1024 hidden layer nodes by training for double the number of batches (9.37 +- 5.40)
* Don't do null move pruning in positions with zugzwang potential (5.70 +- 3.94)
* Remove the depth limit for null move pruning (5.26 +- 3.83)
* Add capture history (4.99 +- 3.69)
* Use strict MVV order before caphist (3.15 +- 2.54)
* Stop using Result to indicate stopped searches (1.80 +- 3.53)
* Simplify down to one killer move (1.75 +- 2.69)
* Fixed a bug that caused en-passant moves to not be generated when
  our king and an enemy rook or queen were on the en-passant file and
  the engine to crash when other engines made these moves. (-0.09 +- 2.24)
  While neutral, this occurred once in every ~1543 games in the SPRT test.
* Fixed a couple of bugs that could (very infrequently) cause time losses.
* Adjust the network scaling factor so that reported scores are less inflated

### Misc

* Ported the transposition table to an implementation that can be shared between threads
* The default transposition table size is now 16MB
* Networks are now stored outside of the repository and downloaded during build if needed
* Fixed a bug that would cause datagen to occasionally generate drawn starting positions
* Fixed a bug that would cause datagen to attribute some games drawn by the 50-move rule as lost/won

## [8.0]

* Network #7, upping the size to 384 hidden layer nodes with a fresh dataset generated using network #6 (62.36 +- 16.50)
* Improve null move pruning reduction formula (29.37 +- 10.50)
* Network #8, upping the size to 512 hidden layer nodes and trained with a larger 1B (unfiltered) FEN dataset (27.07 +- 9.95)
* Network #9, upping the size to 768 hidden layer nodes and trained with an even larger 1B (after filtering) dataset and a WDL proportion of 0.4 (27.05 +- 9.85)
* Add SEE pruning (25.45 +- 9.40)
* Use history gravity and maluses (15.92 +- 7.40)
* Do SEE when picking moves instead of when scoring moves (11.32 +- 6.27)
* Compute checkers in make_move (8.03 +- 5.01)
* Overwrite TT entries from old searches with static eval cache entries (7.85 +- 4.83)
* Cache static eval of positions in the transposition table (5.05 +- 3.69)
* Re-search with zero-window but full depth after raising alpha in a reduced-depth PVS search (2.83 +- 2.22)

### Misc

* Update the minimum Rust version to 1.91
* Remove paste dev dependency
* Remove arrayvec dependency
* Simplify the move picker implementation
* Simplify move scoring by removing sentinel values
* Started tracking the bench in a '.bench' file which can be updated via 'just bench'
* Fixed a datagen bug which caused non-decisive games to always be stored with an outcome of a loss for white
* Switched to using 'tacticals' terminology for non-quiet moves
* Improved 'd eval' command by showing piece contributions similar to Stockfish
* Implement 'go nodes' command
* Start adding a hash suffix to network files
* Derive Copy for ZobristHash
* Added a Rust implementation of OpenBench's SPRT algorithm
* Bench depth was upped from 10 to 12

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

[unreleased]: https://github.com/jgilchrist/chess-engine/compare/v13.0...HEAD
[13.0]: https://github.com/jgilchrist/chess-engine/compare/v12.0..v13.0
[12.0]: https://github.com/jgilchrist/chess-engine/compare/v11.0..v12.0
[11.0]: https://github.com/jgilchrist/chess-engine/compare/v10.0..v11.0
[10.0]: https://github.com/jgilchrist/chess-engine/compare/v9.0..v10.0
[9.0]: https://github.com/jgilchrist/chess-engine/compare/v8.0..v9.0
[8.0]: https://github.com/jgilchrist/chess-engine/compare/v7.0..v8.0
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
