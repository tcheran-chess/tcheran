use crate::{
    chess::{game::Game, moves::Move},
    engine::{
        eval::Eval,
        search::{
            MAX_SEARCH_DEPTH, Reporter, SearchContext, SearchInfo, SearchStats,
            aspiration::aspiration_search, principal_variation::PrincipalVariation,
        },
        util,
    },
};

pub fn search(
    game: &mut Game,
    ctx: &mut SearchContext<'_>,
    pv: &mut PrincipalVariation,
    reporter: &mut impl Reporter,
) -> Option<Move> {
    let mut best_move: Option<Move> = None;
    let mut overall_eval: Option<Eval> = None;

    ctx.max_depth_reached = 0;

    for depth in 1..=MAX_SEARCH_DEPTH {
        if !ctx.time_control.should_start_new_search(depth) {
            break;
        }

        let eval = aspiration_search(game, depth, overall_eval, pv, ctx);
        if ctx.time_control.stopped() {
            break;
        }

        let new_best_move = *pv.first().unwrap_or_else(|| {
            panic!(
                "No PV move at depth {} for position {}",
                depth,
                game.to_fen()
            )
        });

        best_move = Some(new_best_move);
        overall_eval = Some(eval);

        ctx.time_control.update_after_search(new_best_move, depth);

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
                    nodes: ctx.nodes_visited,
                    nodes_per_second: util::metrics::nodes_per_second(
                        ctx.nodes_visited,
                        ctx.time_control.elapsed(),
                    ),
                    tbhits: ctx.tbhits,
                },
            },
        );
    }

    best_move
}
