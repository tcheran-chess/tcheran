use std::{panic::PanicHookInfo, process::ExitCode};

use engine::engine::util::log;

fn get_panic_message(info: &PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        format!("panic occurred: {s:?} {info:?}")
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        format!("panic occurred: {s:?} {info:?}")
    } else {
        format!("{info:?}")
    }
}

fn run() -> ExitCode {
    use engine::engine::uci::UciInputMode;

    let args = std::env::args().collect::<Vec<_>>();
    let uci_input_mode = match args.len() {
        1 => UciInputMode::Stdin,
        2 => {
            let commands = args[1]
                .replace("\\n", "\n")
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            UciInputMode::Commands(commands)
        }
        _ => {
            let binary_name = args[0].clone();
            eprintln!("usage:");
            eprintln!("  {binary_name}                  - run in UCI mode");
            eprintln!(
                "  {binary_name} \"<uci commands>\" - run specific UCI commands and then exit"
            );

            return ExitCode::FAILURE;
        }
    };

    let result = engine::engine::uci::uci(uci_input_mode);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::crashlog(e.clone());
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|info| {
        let panic_message = get_panic_message(info);

        println!("{panic_message}");
        log::crashlog(panic_message);
    }));

    engine::init();
    run()
}
