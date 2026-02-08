use rand::{Rng, SeedableRng, prelude::IndexedRandom, rngs::StdRng};

use crate::{
    chess::game::Game,
    engine::{
        eval::wdl,
        options::EngineOptions,
        search::{
            NullReporter, PersistentState, ThreadData, TimeControl, search,
            time_control::StopControl,
        },
        util::log,
    },
};

const DEFAULT_STARTING_MOVES: usize = 8;

fn random_starting_position(rand: &mut impl Rng) -> Result<Game, ()> {
    let mut game = Game::new();

    // We want to see games where the first non-random move was made by either player, so we want
    // to sometimes make an extra random move so that black can make the first non-random move.
    let black_starts = usize::from(rand.random::<bool>());

    let number_of_random_moves = DEFAULT_STARTING_MOVES + black_starts;

    for _ in 0..number_of_random_moves {
        let moves = game.moves().iter().copied().collect::<Vec<_>>();
        let random_move = moves.choose(rand);

        // We stumbled into a checkmate or draw
        let Some(random_move) = random_move else {
            return Err(());
        };

        game.make_move(*random_move);

        if game.is_draw(0) {
            log::crashlog(format!(
                "Datagen generated a drawn starting position: {}",
                game.to_fen()
            ));

            return Err(());
        }
    }

    // The last move we made may have ended the game.
    let moves = game.moves();
    if moves.is_empty() {
        return Err(());
    }

    Ok(game)
}

fn acceptable_starting_position(rand: &mut impl Rng, state: &mut PersistentState) -> Game {
    const UNBALANCED_STARTING_EVAL: i32 = 1000;

    loop {
        // Skip any games that ended before we got to our starting position
        let Ok(game) = random_starting_position(rand) else {
            continue;
        };

        state.reset();

        // Do a quick search to ensure that we haven't landed in a completely broken (won/lost) position.
        let persistent_state = PersistentState::new(4);

        let (_, eval) = search(
            &game,
            &persistent_state,
            // Non-main so that we don't wait to finish
            &mut ThreadData::new(1),
            TimeControl::Nodes {
                soft: Some(20000),
                hard: Some(20000 * 8),
            },
            &StopControl::new(0),
            &EngineOptions::DEFAULT,
            &NullReporter,
        );

        let normalised_eval = wdl::normalize(eval, &game.board);

        if normalised_eval.0.abs() >= UNBALANCED_STARTING_EVAL {
            continue;
        }

        return game;
    }
}

pub fn generate_random_starting_positions(n: u64, seed: u64, _book: String) -> Vec<Game> {
    let mut rand = StdRng::seed_from_u64(seed);
    let mut persistent_state = PersistentState::new(4);

    let mut games = Vec::new();

    for _ in 0..n {
        let game = acceptable_starting_position(&mut rand, &mut persistent_state);
        games.push(game);
    }

    games
}
