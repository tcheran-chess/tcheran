#![feature(const_trait_impl, const_ops, const_iter, const_index, portable_simd)]

pub mod chess;
pub mod engine;
pub mod util;

#[cfg(test)]
pub mod tests;

pub const ENGINE_NAME: &str = "Tcheran";
pub const ENGINE_VERSION: &str = env!("ENGINE_VERSION");

pub fn init() {
    chess::init();
    engine::init();
}
