use std::sync::atomic::Ordering;

use crate::{
    chess::{game::Game, moves::Move},
    engine::{
        eval::Eval,
        search::{
            ALL_NODE_COUNT, MAX_SEARCH_DEPTH, Reporter, SearchContext, SearchInfo, SearchStats,
            aspiration::aspiration_search, principal_variation::PrincipalVariation,
        },
        util,
    },
};

pub fn search(
    game: &mut Game,
    ctx: &mut SearchContext<'_>,
    pv: &mut PrincipalVariation,
    reporter: &impl Reporter,
) -> Option<Move> {
    let mut best_move: Option<Move> = None;
    let mut overall_eval: Option<Eval> = None;

    ctx.max_depth_reached = 0;

    for depth in 1..=MAX_SEARCH_DEPTH {
        ctx.nodes_visited = 0;

        if !ctx.time_control.should_start_new_search(depth) {
            break;
        }

        let eval = aspiration_search(game, depth, overall_eval, pv, ctx);
        if ctx.time_control.stopped() {
            break;
        }

        let new_best_move = *pv.first().unwrap_or_else(|| {
            panic!("No PV move at depth {} for position {}", depth, game.to_fen())
        });

        best_move = Some(new_best_move);
        overall_eval = Some(eval);

        ctx.time_control.update_after_search(new_best_move, depth);
        ALL_NODE_COUNT.fetch_add(ctx.nodes_visited, Ordering::Relaxed);
        let all_node_count = ALL_NODE_COUNT.load(Ordering::Relaxed);

        reporter.report_search_progress(
            game,
            SearchInfo {
                depth,
                seldepth: ctx.max_depth_reached,
                eval,
                pv: pv.clone(),
                hashfull: ctx.tt.occupancy(),
                stats: SearchStats {
                    time: ctx.time_control.elapsed(),
                    nodes: all_node_count,
                    nodes_per_second: util::metrics::nodes_per_second(
                        all_node_count,
                        ctx.time_control.elapsed(),
                    ),
                    tbhits: ctx.tbhits,
                },
            },
        );
    }

    best_move
}
