use crate::{
    chess::game::Game,
    engine::search::{
        MAX_SEARCH_DEPTH, Reporter, SearchContext, SearchInfo, SearchResult, SearchStats,
        aspiration::aspiration_search, principal_variation::PrincipalVariation,
    },
};

pub fn search(
    game: &mut Game,
    ctx: &mut SearchContext<'_>,
    reporter: &impl Reporter,
) -> Option<SearchResult> {
    let mut pv = PrincipalVariation::new();
    let mut result: Option<SearchResult> = None;

    ctx.max_depth_reached = 0;

    for depth in 1..=MAX_SEARCH_DEPTH {
        if !ctx.time_control.should_start_new_search(depth, ctx) {
            break;
        }

        ctx.root_depth = depth;

        let previous_eval = result.as_ref().map(|r| r.eval);
        let eval = aspiration_search(game, depth, previous_eval, &mut pv, ctx);

        if ctx.time_control.stopped() {
            break;
        }

        let new_best_move = *pv.first().unwrap_or_else(|| {
            panic!("No PV move at depth {} for position {}", depth, game.to_fen())
        });

        result = Some(SearchResult {
            best_move: new_best_move,
            eval,
            pv: pv.clone(),
        });

        ctx.completed_depth = depth;
        ctx.time_control
            .update_after_search(new_best_move, depth, ctx.nodes_visited.get());

        reporter.report_search_progress(SearchInfo {
            game,
            eval,
            pv: &pv,
            stats: SearchStats::from_ctx(ctx),
        });
    }

    result
}
