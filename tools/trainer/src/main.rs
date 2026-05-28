mod bullet_extensions;
mod trainer;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        println!("usage: <net_name>");
        std::process::exit(1);
    }

    let net_name = &args[1];
    trainer::run(net_name);
}
