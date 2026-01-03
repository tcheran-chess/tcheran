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
    reporter: &impl Reporter,
) -> Option<(Move, Eval)> {
    let mut pv = PrincipalVariation::new();
    let mut result: Option<(Move, Eval)> = None;

    ctx.max_depth_reached = 0;

    for depth in 1..=MAX_SEARCH_DEPTH {
        if !ctx.time_control.should_start_new_search(depth, ctx) {
            break;
        }

        ctx.root_depth = depth;

        let previous_eval = result.map(|r| r.1);
        let eval = aspiration_search(game, depth, previous_eval, &mut pv, ctx);

        if ctx.time_control.stopped() {
            break;
        }

        let new_best_move = *pv.first().unwrap_or_else(|| {
            panic!("No PV move at depth {} for position {}", depth, game.to_fen())
        });

        result = Some((new_best_move, eval));

        ctx.time_control.update_after_search(new_best_move, depth);

        reporter.report_search_progress(
            game,
            SearchInfo {
                game: game.clone(),
                depth,
                seldepth: ctx.max_depth_reached,
                eval,
                pv: pv.clone(),
                hashfull: ctx.tt.occupancy(),
                stats: SearchStats {
                    time: ctx.time_control.elapsed(),
                    nodes: ctx.nodes_visited.get_global(),
                    nodes_per_second: util::metrics::nodes_per_second(
                        ctx.nodes_visited.get_global(),
                        ctx.time_control.elapsed(),
                    ),
                    tbhits: ctx.tbhits.get_global(),
                },
            },
        );
    }

    result
}
