use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    ops::ControlFlow,
    path::PathBuf,
    time::Instant,
};

use anyhow::{Result, bail};
use clap::Args;
use engine::chess::{game::Game, moves::Move, player::Player, san};
use pgn_reader::{RawComment, RawTag, Reader, SanPlus, Visitor};
use viriformat::{
    chess::board::{Board, DrawType, GameOutcome, WinType},
    dataformat::Game as ViriGame,
};

use crate::viriformat_ext::ToViriExt;

#[derive(Debug, Args)]
pub struct ConvertOptions {
    pub input: PathBuf,
}

struct PgnToViri;

#[derive(Default)]
struct Tags {
    fen: Option<String>,
    outcome: Option<GameOutcome>,
}

struct State {
    game: Game,
    virigame: ViriGame,

    moves: Vec<Move>,
    comments_seen: usize,
}

impl Visitor for PgnToViri {
    type Tags = Tags;
    type Movetext = State;
    type Output = ViriGame;

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(Tags::default())
    }

    fn tag(
        &mut self,
        tags: &mut Self::Tags,
        name: &[u8],
        value: RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        let name = String::from_utf8_lossy(name).to_string();
        let value = String::from_utf8_lossy(value.as_bytes()).to_string();

        if name == "Result" {
            tags.outcome.replace(match value.as_str() {
                "1-0" => GameOutcome::WhiteWin(WinType::Mate),
                "0-1" => GameOutcome::BlackWin(WinType::Mate),
                "1/2-1/2" => GameOutcome::Draw(DrawType::Adjudication),
                _ => panic!("Malformed PGN: Unknown result: {value}"),
            });
        }

        if name == "FEN" {
            tags.fen.replace(value);
        }

        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        let fen = tags.fen.expect("Malformed PGN: No FEN");
        let outcome = tags.outcome.expect("Malformed PGN: No Result");

        let game = Game::from_fen(&fen).unwrap();

        let mut viriboard = Board::new();

        viriboard
            .set_from_fen(&game.to_fen())
            .expect("Should be able to construct game from FEN");

        let mut virigame = ViriGame::new(&viriboard);

        // Uncomment for compatibility with Pawnocchio's output
        // virigame = ViriGame {
        //     initial_position: viriboard.to_marlinformat(0, 0, 164),
        //     moves: vec![],
        // };

        virigame.set_outcome(outcome);

        ControlFlow::Continue(State {
            game,
            virigame,
            moves: Vec::new(),
            comments_seen: 0,
        })
    }

    fn san(
        &mut self,
        movetext: &mut Self::Movetext,
        san_plus: SanPlus,
    ) -> ControlFlow<Self::Output> {
        let san = san_plus.san.to_string();

        let mv = san::parse_move(&movetext.game, &san).unwrap_or_else(|_| {
            panic!("Malformed move: {san} in position {}", movetext.game.to_fen())
        });

        movetext.moves.push(mv);

        ControlFlow::Continue(())
    }

    fn comment(
        &mut self,
        movetext: &mut Self::Movetext,
        comment: RawComment<'_>,
    ) -> ControlFlow<Self::Output> {
        movetext.comments_seen += 1;
        assert_eq!(
            movetext.comments_seen,
            movetext.moves.len(),
            "Malformed PGN: Moves found without comments"
        );

        let comment = String::from_utf8_lossy(comment.as_bytes()).to_string();
        let mv = *movetext.moves.last().expect("Malformed PGN: No moves");

        // Assumes the structure of the comments is +score.scoredecimal/depth
        let score = comment
            .split_once('/')
            .expect("Move comment should contain '/'")
            .0;

        #[expect(clippy::cast_possible_truncation, reason = "Eval values won't be truncated")]
        let score = if score.contains('M') {
            let is_mated = score.contains('-');
            if is_mated { -i16::MAX } else { i16::MAX }
        } else {
            (score.parse::<f64>().unwrap() * 100.0) as i16
        };

        let score = if movetext.game.player == Player::White {
            score
        } else {
            -score
        };

        movetext.game.make_move(mv);
        movetext.virigame.add_move(mv.to_viri(), score);

        ControlFlow::Continue(())
    }

    fn end_game(&mut self, movetext: Self::Movetext) -> Self::Output {
        movetext.virigame
    }
}

pub fn run(options: &ConvertOptions) -> Result<()> {
    let output_file_name = options.input.with_extension("viri");
    if output_file_name.exists() {
        bail!("{} already exists, stopping to prevent corruption", output_file_name.display());
    }

    let file = File::open(&options.input)?;
    let reader = BufReader::new(file);
    let mut writer = BufWriter::new(File::create(output_file_name)?);

    let mut pgn_reader = Reader::new(reader);

    let mut games = 0usize;
    let mut positions = 0usize;
    let now = Instant::now();

    for game in pgn_reader.read_games(&mut PgnToViri) {
        let game = game?;
        game.serialise_into(&mut writer)?;
        games += 1;
        positions += game.len();

        if games.is_multiple_of(1_048_576) {
            println!("{games} games converted");
        }
    }

    let elapsed = now.elapsed();

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "Approximate calculation"
    )]
    let positions_per_second = (positions as f64 / elapsed.as_secs_f64()) as u64;

    println!(
        "Converted {games} games ({positions} positions) in {elapsed:?} ({positions_per_second} pos/s)"
    );

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io;

    use pgn_reader::Reader;

    use super::*;

    #[test]
    fn test_pgn_parsing_doesnt_panic() {
        engine::init();

        let sample_pgn = r#"
[Event "Fastchess Tournament"]
[Site "?"]
[Date "2026.01.15"]
[Round "2"]
[White "Tcheran-base"]
[Black "Tcheran-dev"]
[Result "0-1"]
[FEN "r1b1kbnr/pp1ppppp/1qn5/8/3p4/4P2P/PPPKNPP1/RNBQ1B1R b kq - 1 5"]
[TimeControl "-"]
[ScaleFactor "0.33114457805392294"]

Qa5+ {+2.43/6} c3 {-2.34/7} dxe3+ {+2.38/7} Kxe3 {-2.28/6} d5 {+2.12/6} Kf3 {-2.19/7} e5 {+2.04/7} g3 {-2.25/7} Nf6 {+2.24/7} Kg2 {-2.21/8} Qc7 {+2.31/7} Be3 {-2.07/7} Be7 {+2.22/6} Nd2 {-1.97/7} O-O {+2.13/7} Kh2 {-2.06/7} Bf5 {+2.07/7} Bg2 {-2.09/7} Rad8 {+2.04/7} f4 {-2.03/6} d4 {+2.03/6} cxd4 {-2.02/7} exd4 {+1.97/7} Bf2 {-2.11/8} Bc5 {+2.03/7} Nb3 {-2.18/7} Bb6 {+2.18/7} Nexd4 {-2.03/7} Nxd4 {+2.00/6} Bxd4 {-1.61/6} Qc4 {+2.48/6} Rc1 {-2.54/6} Qa4 {+2.91/6} Bxb7 {-3.93/6} Bxd4 {+3.93/7} Nxd4 {-4.40/7} Qxd4 {+4.29/9} Qxd4 {-4.20/7} Rxd4 {+3.92/8} g4 {-3.55/7} Rd2+ {+3.66/7} Kg3 {-3.58/7} Rxb2 {+3.66/8} Rc7 {-3.52/8} Be4 {+3.44/6} Bxe4 {-3.58/7} Nxe4+ {+3.31/6} Kh4 {-3.56/6} Rxa2 {+3.40/6} Re1 {-3.61/7} Ra4 {+3.64/7} Rec1 {-3.66/6} Nf6 {+3.77/7} R1c5 {-3.78/6} h6 {+3.65/6} f5 {-3.79/7} a6 {+3.72/7} Rc8 {-3.80/7} a5 {+3.89/7} R8c7 {-3.71/7} Rb4 {+3.79/7} Rxa5 {-3.83/8} h5 {+3.67/8} Rca7 {-3.78/7} hxg4 {+3.73/9} Ra4 {-3.91/7} Rxa4 {+3.96/10} Rxa4 {-3.70/9} gxh3 {+3.35/8} Kxh3 {-3.19/8} Re8 {+3.17/7} Ra3 {-3.12/8} Rd8 {+3.23/8} Ra4 {-3.14/8} Kf8 {+3.07/7} Kg3 {-3.04/7} Rd3+ {+3.07/6} Kf4 {-3.02/7} Nd5+ {+3.12/7} Ke5 {-2.95/6} f6+ {+3.64/9} Ke4 {-3.64/6} Nc3+ {+3.80/10} Kxd3 {-4.39/10} Nxa4 {+4.29/11} Kd4 {-4.51/9} Ke7 {+4.39/10} Ke4 {-4.65/8} Kd6 {+4.71/9} Kd3 {-5.64/9} Ke5 {+5.40/10} Ke3 {-5.81/8} Nc5 {+5.62/10} Kf2 {-6.59/8} Kxf5 {+5.97/9} Kf3 {-6.72/8} Ne4 {+6.52/10} Kg2 {-6.64/9} g5 {+6.79/10} Kh2 {-7.04/8} Kf4 {+7.02/10} Kg1 {-7.27/9} g4 {+7.51/12} Kf1 {-7.43/9} Kf3 {+7.82/12} Kg1 {-8.97/10} g3 {+8.10/12} Kh1 {-M12/10} f5 {+12.89/13} Kg1 {-M8/9} Nf2 {+13.96/10} Kf1 {-7.20/9} g2+ {+13.79/12} Ke1 {-13.65/7} g1=Q+ {+14.20/10} Kd2 {-13.60/7} Qg4 {+13.82/10} Kc3 {-13.74/8} Qe4 {+14.08/9} Kd2 {-13.53/7} Qc4 {+M3/9} Ke1 {-M2/11} Qc1# {+M1/11} 0-1

[Event "Fastchess Tournament"]
[Site "?"]
[Date "2026.01.15"]
[Round "1"]
[White "Tcheran-dev"]
[Black "Tcheran-base"]
[Result "0-1"]
[FEN "rnbqkbnr/ppp2p2/4p2p/3p2p1/2P1P1Q1/7N/PP1P1PPP/RNB1KB1R w KQkq - 0 5"]
[TimeControl "-"]
[ScaleFactor "0.33114457805392294"]

d3 {-1.14/6} Nc6 {+1.16/6} Qe2 {-1.13/7} Nf6 {+1.10/7} f3 {-1.11/6} Nd4 {+0.89/6} Qd1 {-0.91/7} c6 {+1.04/7} e5 {-1.03/6} Nd7 {+1.04/8} f4 {-2.15/6} gxf4 {+1.99/8} Bxf4 {-2.15/7} Bg7 {+1.99/7} Qg4 {-2.15/7} Kf8 {+2.07/7} Qg3 {-2.06/7} Nc2+ {+1.82/7} Kd1 {-2.00/6} Nxa1 {+1.77/8} Kc1 {-2.01/7} Qa5 {+2.42/8} Nc3 {-2.44/7} b5 {+2.32/8} Kb1 {-2.25/8} b4 {+2.24/8} Ne2 {-2.11/7} dxc4 {+2.09/7} dxc4 {-1.99/8} Bxe5 {+2.20/7} Kxa1 {-2.25/7} Bxf4 {+2.16/7} Nexf4 {-2.14/8} b3 {+2.00/7} a3 {-2.07/8} e5 {+1.97/7} Ne6+ {-1.54/8} Ke7 {+1.46/6} Ng7 {-1.53/7} Qd2 {+1.28/6} Bd3 {-1.42/7} Bb7 {+0.67/7} Be4 {-0.76/6} Rad8 {+0.38/7} Nf5+ {-0.18/5} Ke6 {+0.57/7} Qh4 {+0.16/8} f6 {+0.03/6} g4 {-1.00/6} Nc5 {+1.12/8} Nf2 {-2.03/6} Qf4 {+1.70/8} Bb1 {-2.07/8} Kd7 {+1.89/8} Re1 {-1.87/7} Kc7 {+1.44/6} Ne4 {-2.20/7} Nxe4 {+1.65/8} Rxe4 {-2.01/7} Qg5 {+1.56/9} Qe1 {-1.57/7} Qd2 {+1.71/8} Qg3 {-1.50/7} Bc8 {+1.94/6} Ne7 {-2.37/6} Be6 {+2.38/8} Qf3 {-2.96/6} Rhe8 {+2.27/7} Nxc6 {-1.95/7} Qd1 {+1.77/7} Nxe5 {-3.55/7} Qxf3 {+3.51/9} Nxf3 {-4.13/8} Rd1 {+3.30/8} Rf4 {-3.81/7} Rf1 {+3.56/8} h3 {-3.47/7} Bd7 {+5.00/9} Rxf6 {-5.20/8} Re3 {+5.40/9} c5 {-7.24/10} Rexf3 {+6.89/10} Rxf3 {-7.20/9} Rxf3 {+6.91/10} c6 {-6.35/9} Bxc6 {+8.17/11} a4 {-7.53/8} Rf4 {+7.98/11} Bd3 {-8.28/7} Rxa4+ {+8.47/11} Kb1 {-8.85/7} Be4 {+9.02/12} Bxe4 {-9.14/9} Rxe4 {+9.11/12} Kc1 {-10.30/9} Rd4 {+9.24/11} h4 {-9.43/9} Rxg4 {+9.27/9} Kd2 {-9.05/8} Rg3 {+9.50/9} Ke2 {-9.89/7} Rg2+ {+10.04/10} Kd3 {-10.99/7} Rxb2 {+10.15/10} Kc4 {-11.47/9} Kc6 {+11.00/11} Kb4 {-11.85/8} h5 {+12.44/12} Kc3 {-14.24/8} Rb1 {+13.18/10} Kd4 {-14.79/8} Rc1 {+14.12/10} Ke5 {-15.30/8} Rc5+ {+14.85/8} Kf6 {-16.71/6} a5 {+16.57/10} Kf7 {-17.23/8} b2 {+18.76/9} Kg8 {-19.22/8} b1=Q {+21.71/11} Kg7 {-19.67/8} Qe4 {+22.03/9} Kf7 {-22.09/7} Qxh4 {+23.12/10} Kg7 {-22.44/7} Qg4+ {+22.54/8} Kf7 {-M6/8} Rf5+ {+M5/9} Ke7 {-M4/10} 0-1
"#;

        let mut reader = Reader::new(io::Cursor::new(&sample_pgn));

        for _ in reader.read_games(&mut PgnToViri) {
            // Left empty
        }
    }
}
