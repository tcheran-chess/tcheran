#[allow(
    unused,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::unreadable_literal,
    clippy::use_self
)]
mod bindings;
mod tablebases;

pub use tablebases::*;
