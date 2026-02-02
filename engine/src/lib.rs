pub mod chess;
pub mod engine;

#[cfg(test)]
pub mod tests;

pub const ENGINE_NAME: &str = "Tcheran";
pub const ENGINE_VERSION: &str = env!("ENGINE_VERSION");

pub fn init() {
    chess::init();
    engine::init();
}
