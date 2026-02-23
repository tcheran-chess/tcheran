macro_rules! parameters {
    (
        $(
            ($name:ident, $ty:ty, $default:expr, $min:expr, $max:expr)
        ),* $(,)?
    ) => {
        #[cfg(not(feature = "spsa"))]
        mod default_params {
            $(
                pub const fn $name() -> $ty {
                    $default
                }
            )*
        }

        #[cfg(not(feature = "spsa"))]
        pub use default_params::*;

        #[cfg(feature = "spsa")]
        mod spsa_params {
            #[expect(non_upper_case_globals, reason = "Only exposed internally")]
            mod values {
            $(
                pub static mut $name: $ty = $default;
            )*
            }

            pub mod getters {
            $(
                pub const fn $name() -> $ty {
                    unsafe { super::values::$name }
                }
            )*
            }

            pub fn spsa_params() -> Vec<crate::engine::uci::options::UciOption> {
                vec![
                    $(
                        crate::engine::uci::options::UciOption::spin(stringify!($name), |_refs, value| {
                          unsafe { values::$name = value.into() };
                          crate::engine::tuning::on_option_change()
                       })
                       .default($default)
                       .with_bounds($min, $max)
                       .build(),
                    )*
                ]
            }
        }

        #[cfg(feature = "spsa")]
        pub use spsa_params::{spsa_params, getters::*};
    };
}

macro_rules! non_tunable_parameters {
    (
        $(
            ($name:ident, $ty:ty, $value:expr)
        ),* $(,)?
    ) => {
        $(
            pub const fn $name() -> $ty {
                $value
            }
        )*
    };
}

#[cfg(feature = "spsa")]
pub fn on_option_change() {
    use super::params::*;

    // Re-initialise the LMR table, which depends on parameters
    crate::engine::search::tables::lmr_table::init();

    // Re-initialise the SEE piece value array since it contains multiple parameters
    crate::engine::see::init_see_values(
        see_pawn_value(),
        see_knight_value(),
        see_bishop_value(),
        see_rook_value(),
        see_queen_value(),
    );
}

pub(crate) use non_tunable_parameters;
pub(crate) use parameters;
