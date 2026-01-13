#![expect(clippy::similar_names, reason = "Maintaining naming from the original")]
#![expect(clippy::cast_precision_loss, reason = "Imprecise calculations are fine")]

use std::process::ExitCode;

use argmin::{
    core::{CostFunction, Error, Executor},
    solver::brent::BrentRoot,
};
use clap::Parser;
use statrs::distribution::{ContinuousCDF, Normal};

// # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # #
// #                                                                             #
// #   OpenBench is a chess engine testing framework authored by Andrew Grant.   #
// #   <https://github.com/AndyGrant/OpenBench>           <andrew@grantnet.us>   #
// #                                                                             #
// #   OpenBench is free software: you can redistribute it and/or modify         #
// #   it under the terms of the GNU General Public License as published by      #
// #   the Free Software Foundation, either version 3 of the License, or         #
// #   (at your option) any later version.                                       #
// #                                                                             #
// #   OpenBench is distributed in the hope that it will be useful,              #
// #   but WITHOUT ANY WARRANTY; without even the implied warranty of            #
// #   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the             #
// #   GNU General Public License for more details.                              #
// #                                                                             #
// #   You should have received a copy of the GNU General Public License         #
// #   along with this program.  If not, see <http://www.gnu.org/licenses/>.     #
// #                                                                             #
// # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # #
//
// Note that the below code is not OpenBench's code but rather a direct Rust translation
// of the code, so the above license is provided.

#[derive(Parser)]
struct Cli {
    ll: f64,
    ld: f64,
    dd: f64,
    dw: f64,
    ww: f64,

    #[clap(long, allow_hyphen_values = true)]
    elo0: f64,

    #[clap(long, allow_hyphen_values = true)]
    elo1: f64,
}

type Penta = [f64; 5];
type Pdf = [(f64, f64); 5];

fn sprt(mut penta: Penta, elo0: f64, elo1: f64) -> f64 {
    // ## Implements https://hardy.uhasselt.be/Fishtest/normalized_elo_practical.pdf

    // # Ensure no division by 0 issues
    penta = penta
        .into_iter()
        .map(|v| v.max(1e-3))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // Partial computation of Normalized t-value
    let nelo_divided_by_nt = 800.0 / 10.0f64.ln();
    let (nt0, nt1) = (elo0 / nelo_divided_by_nt, elo1 / nelo_divided_by_nt);
    let (t0, t1) = (nt0 * 2.0f64.sqrt(), nt1 * 2.0f64.sqrt());

    // Number of game-pairs, and the PDF of Pntml(0-2) expressed as (0-1)
    let n = penta.iter().sum::<f64>();
    let pdf: Pdf = penta
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64 / 4.0, v / n))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // Pdf given each normalized t-value, and then the LLR process for each
    let (pdf0, pdf1) = (mle_tvalue(pdf, 0.5, t0), mle_tvalue(pdf, 0.5, t1));

    let mle_pdf = (0..5)
        .map(|i| ((pdf1[i].1).ln() - (pdf0[i].1).ln(), pdf[i].1))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    n * stats(mle_pdf).0
}

fn elo(penta: Penta) -> (f64, f64, f64) {
    let n = penta.iter().sum::<f64>();
    assert!(n > 0.0);

    // Converts index to the points outcome
    let div = 4.0f64;

    let mu = penta
        .iter()
        .enumerate()
        .map(|(f, v)| (f as f64 / div) * v)
        .sum::<f64>()
        / n;

    let var = penta
        .iter()
        .enumerate()
        .map(|(f, v)| ((f as f64 / div) - mu).powi(2) * v)
        .sum::<f64>()
        / n;

    let normdist = Normal::standard();
    let mu_min = mu + normdist.inverse_cdf(0.025) * var.sqrt() / n.sqrt();
    let mu_max = mu + normdist.inverse_cdf(0.975) * var.sqrt() / n.sqrt();

    let elo = logistic_elo(mu);
    let elo_lower = logistic_elo(mu_min);
    let elo_upper = logistic_elo(mu_max);

    (elo_lower, elo, elo_upper)
}

fn mle_tvalue(pdfhat: Pdf, refv: f64, s: f64) -> Pdf {
    let mut pdf_mle = uniform(pdfhat);

    for _ in 0..10 {
        let pdf_ = pdf_mle;
        let (mu, var) = stats(pdf_mle);

        let sigma = var.sqrt();

        let pdf1: Pdf = pdfhat
            .iter()
            .map(|(ai, pi)| {
                (
                    ai - refv
                        - s * sigma * ((mu - ai) / sigma).mul_add((mu - ai) / sigma, 1.0) / 2.0,
                    *pi,
                )
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let x = secular(pdf1);

        pdf_mle = (0..5)
            .map(|i| (pdfhat[i].0, pdfhat[i].1 / x.mul_add(pdf1[i].0, 1.0)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        if (0..5)
            .map(|i| (pdf_[i].1 - pdf_mle[i].1).abs())
            .max_by(f64::total_cmp)
            .unwrap()
            < 1e-9
        {
            break;
        }
    }

    pdf_mle
}

fn stats(pdf: Pdf) -> (f64, f64) {
    let epsilon = 1e-6;

    for (_, i) in &pdf {
        assert!(-epsilon <= *i);
        assert!(*i <= 1.0 + epsilon);
    }

    let n = pdf.iter().map(|(_, prob)| *prob).sum::<f64>();
    assert!((n - 1.0).abs() < epsilon);

    let s = pdf.iter().map(|(value, prob)| prob * value).sum::<f64>();

    let var = pdf
        .iter()
        .map(|(value, prob)| prob * (value - s).powi(2))
        .sum::<f64>();

    (s, var)
}

struct BrentFn {
    pdf: Pdf,
}

impl CostFunction for BrentFn {
    type Param = f64;
    type Output = f64;

    fn cost(&self, x: &Self::Param) -> Result<Self::Output, Error> {
        Ok(self
            .pdf
            .iter()
            .map(|(ai, pi)| pi * ai / (1.0 + x * ai))
            .sum::<f64>())
    }
}

fn secular(pdf: Pdf) -> f64 {
    let epsilon = 1e-9;
    let (v, w) = (pdf[0].0, pdf[4].0);

    assert!(v * w < 0.0);

    let l = -1.0 / w;
    let u = -1.0 / v;

    let cost = BrentFn { pdf };
    let brent = BrentRoot::new(l + epsilon, u - epsilon, 2e-12);
    let result = Executor::new(cost, brent).run().unwrap();

    assert!(result.state.termination_status.terminated());

    result.state.best_param.unwrap()
}

fn uniform(pdf: Pdf) -> Pdf {
    let n = 5.0;

    pdf.iter()
        .map(|(ai, _)| (*ai, 1.0 / n))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn logistic_elo(x: f64) -> f64 {
    let x = x.clamp(1e-3, 1.0f64 - 1.0e-3);
    -400.0 * (1.0 / x - 1.0).log10()
}

const CONSOLE_GREEN: &str = "\x1b[42m";
const CONSOLE_RED: &str = "\x1b[41m";
const CONSOLE_RESET: &str = "\x1b[0m";

pub fn main() -> ExitCode {
    let cli = Cli::parse();

    let penta = [cli.ll, cli.ld, cli.dd, cli.dw, cli.ww];
    let elo0 = cli.elo0;
    let elo1 = cli.elo1;

    let alpha = 0.05f64;
    let beta = 0.1f64;
    let lowerllr = (beta / (1.0 - alpha)).ln();
    let upperllr = ((1.0 - beta) / alpha).ln();

    let (elo_lower, elo, _elo_upper) = elo(penta);
    let elo_bound = elo - elo_lower;

    let llr = sprt(penta, elo0, elo1);
    let mut decided = false;

    if llr > upperllr {
        println!("{CONSOLE_GREEN}PASSED{CONSOLE_RESET}");
        decided = true;
    }

    if llr < lowerllr {
        println!("{CONSOLE_RED}FAILED{CONSOLE_RESET}");
        decided = true;
    }

    println!("LLR: {llr:.2} ({lowerllr:0.2}, {upperllr:0.2}) [{elo0:.1}, {elo1:.1}]");
    println!("Elo: {elo:.2} +- {elo_bound:.2}");

    if !decided {
        let total_pairs = penta.iter().sum::<f64>();

        let ll_percent = cli.ll / total_pairs;
        let ld_percent = cli.ld / total_pairs;
        let dd_percent = cli.dd / total_pairs;
        let dw_percent = cli.dw / total_pairs;
        let ww_percent = cli.ww / total_pairs;

        let mut speculative_pairs = total_pairs;
        loop {
            speculative_pairs += 1.0;
            let speculative_penta = [
                ll_percent * speculative_pairs,
                ld_percent * speculative_pairs,
                dd_percent * speculative_pairs,
                dw_percent * speculative_pairs,
                ww_percent * speculative_pairs,
            ];

            let llr = sprt(speculative_penta, elo0, elo1);
            if llr > upperllr || llr < lowerllr {
                let additional_pairs = speculative_pairs - total_pairs;
                let additional_games = additional_pairs * 2.0;
                let speculative_games = speculative_pairs * 2.0;
                println!(
                    "{additional_pairs} more pairs required ({additional_games} more games for {speculative_games} total) for termination assuming no Elo change"
                );
                break;
            }
        }
    }

    ExitCode::SUCCESS
}
