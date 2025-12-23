#[cfg(not(feature = "spsa"))]
pub fn print_spsa_input() {
    println!("spsa feature is not enabled");
}

#[cfg(feature = "spsa")]
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

                spsa_step,
            } => {
                let spsa_step = spsa_step.unwrap_or_else(|| panic!("No SPSA step set for {name}"));

                println!("{name}, int, {default}, {min}, {max}, {spsa_step:.1}, 0.002");
            }
            _ => panic!("Invalid SPSA option: {name}"),
        }
    }
}
