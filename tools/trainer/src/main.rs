cfg_select! {
    target_os = "macos" => {
        mod trainer {
            pub fn run() {
                println!("Cannot run on macOS");
            }
        }
    }
    _ => {
        mod bullet_extensions;
        mod trainer;
    }
}

fn main() {
    trainer::run();
}
