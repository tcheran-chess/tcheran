use std::{
    backtrace::{Backtrace, BacktraceStatus},
    fmt::Write,
    panic::PanicHookInfo,
    process::ExitCode,
};

use engine::engine::util::log;

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::capture();
        let panic_message = get_panic_message(info, &backtrace);

        println!("{panic_message}");
        log::crashlog(panic_message);
    }));

    engine::init();
    run()
}

fn run() -> ExitCode {
    use engine::engine::uci::UciInputMode;

    let args = std::env::args().collect::<Vec<_>>();
    let uci_input_mode = if args.len() == 1 {
        UciInputMode::Stdin
    } else {
        let commands = args[1..]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        UciInputMode::Commands(commands)
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

fn get_panic_message(info: &PanicHookInfo<'_>, backtrace: &Backtrace) -> String {
    let mut message = if let Some(s) = info.payload().downcast_ref::<&str>() {
        format!("panic occurred: {s:?} {info:?}")
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        format!("panic occurred: {s:?} {info:?}")
    } else {
        format!("{info:?}")
    };

    if backtrace.status() == BacktraceStatus::Captured {
        let _ = write!(message, "\n{}", format_backtrace(backtrace));
    }

    message
}

fn format_backtrace(backtrace: &Backtrace) -> String {
    if backtrace.status() != BacktraceStatus::Captured {
        return "<no backtrace>".to_string();
    }

    let lines = format!("{backtrace:#?}");
    let lines = lines.lines().collect::<Vec<_>>();
    let mut lines = lines.iter();

    // Skip the first line: Backtrace [
    lines.next();

    let is_not_our_line = |s: &str| {
        s.contains("std::")
            || s.contains("core::")
            || s.contains("<alloc::")
            || s.contains("__rustc")
            || s.contains("__rust_try")
            || s.contains("__pthread")
            // Always exclude the frame from the panic handling code
            || s.contains("engine::main::{{closure}}")
    };

    let mut message = String::new();

    for line in lines {
        if !is_not_our_line(line) {
            let _ = write!(message, "\n{line}");
        }
    }

    message
}
