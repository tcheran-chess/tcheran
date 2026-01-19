use std::time::Duration;

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "This is intended to be approximate so we don't care about this being lossy"
)]
pub fn nodes_per_second(nodes: u64, elapsed_time: Duration) -> u64 {
    (nodes as f64 / elapsed_time.as_secs_f64()) as u64
}

pub enum UnitPrefix {
    None,
    Kilo,
    Mega,
    Giga,
    Tera,
}

#[expect(
    clippy::cast_precision_loss,
    reason = "This is intended to be approximate so we don't care about this being lossy"
)]
pub fn unit_suffix(v: u64) -> (String, UnitPrefix) {
    use UnitPrefix::*;

    let vf = v as f64;

    match v {
        0..1000 => (v.to_string(), None),
        1000..10_000 => (format!("{:.1}", vf / 1000.0), Kilo),
        10_000..1_000_000 => (format!("{:.0}", vf / 1000.0), Kilo),
        1_000_000..10_000_000 => (format!("{:.1}", vf / 1_000_000.0), Mega),
        10_000_000..1_000_000_000 => (format!("{:.0}", vf / 1_000_000.0), Mega),
        1_000_000_000..10_000_000_000 => (format!("{:.1}", vf / 1_000_000_000.0), Giga),
        10_000_000_000..1_000_000_000_000 => (format!("{:.0}", vf / 1_000_000_000.0), Giga),
        1_000_000_000_000..10_000_000_000_000 => (format!("{:.1}", vf / 1_000_000_000_000.0), Tera),
        10_000_000_000_000.. => (format!("{:.0}", vf / 1_000_000_000_000.0), Tera),
    }
}
