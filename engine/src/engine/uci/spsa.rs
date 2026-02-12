pub fn print_spsa_input() {
    use crate::engine::uci::options::{UciOption, UciOptionType};

    let options = crate::engine::params::spsa_params();

    for UciOption { name, t } in options {
        match t {
            UciOptionType::Spin {
                default,
                min,
                max,
                set_fn: _,
            } => {
                let c_end = (max as f64 - min as f64) / 20.0;
                let r_end = 0.002 / (c_end.min(0.5) / 0.5);

                println!("{name}, int, {default}, {min}, {max}, {c_end:.2}, {r_end}");
            }
            _ => panic!("Invalid SPSA option: {name}"),
        }
    }
}
