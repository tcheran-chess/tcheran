use crate::{
    chess::Game,
    engine::search::{
        MAX_SEARCH_DEPTH, Reporter, SearchContext, SearchResult, SearchStats,
        aspiration::aspiration_search, principal_variation::PrincipalVariation, types::Depth,
    },
};

pub fn search(
    game: &mut Game,
    ctx: &mut SearchContext<'_>,
    reporter: &dyn Reporter,
) -> SearchResult {
    let mut pv = PrincipalVariation::new();
    let mut result: Option<SearchResult> = None;

    ctx.max_depth_reached = 0;

    for depth in 1..=MAX_SEARCH_DEPTH {
        let depth = Depth::new(depth);

        if !ctx.time_control.should_start_new_search(depth, ctx) {
            break;
        }

        ctx.root_depth = depth;

        let previous_eval = result.as_ref().map(|r| r.eval);
        let eval = aspiration_search(game, depth, previous_eval, &mut pv, ctx);

        if ctx.time_control.stopped() {
            ctx.was_hard_stopped = true;
            break;
        }

        let new_best_move = *pv.first().unwrap_or_else(|| {
            panic!("No PV move at depth {} for position {}", depth, game.to_fen())
        });

        ctx.completed_depth = depth;
        ctx.time_control
            .update_after_search(new_best_move, depth, ctx.nodes.get());

        let this_result = SearchResult {
            best_move: new_best_move,
            eval,
            pv: pv.clone(),
            stats: SearchStats::from_ctx(ctx),
        };

        if !ctx.options.minimal {
            reporter.report_search_progress(game, &this_result);
        }

        result = Some(this_result);
    }

    result.expect("Should always end iterative deepening with a result")
}
