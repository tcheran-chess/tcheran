use std::path::Path;

mod bullet_extensions;
mod data;
mod inputs;
mod trainer;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let name = std::env::args().nth(1).expect("missing network name");
    trainer::run(dir, &name);
}
