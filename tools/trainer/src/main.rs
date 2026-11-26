mod bullet_extensions;
mod data;
mod trainer;

fn main() {
    let name = std::env::args().nth(1).expect("missing network name");
    trainer::run(&name);
}
