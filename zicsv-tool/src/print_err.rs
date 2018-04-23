use failure;

pub fn print_error(error: &failure::Error) {
    eprintln!("Error:");

    let error_backtrace = format!("{}", error.backtrace());
    let mut duplicate_error_backtrace = false;
    for cause in error.causes() {
        eprintln!("    {}", cause);
        let _ = cause.backtrace().map(|backtrace| {
            let backtrace = format!("{}", backtrace);
            if !backtrace.is_empty() {
                if backtrace == error_backtrace {
                    duplicate_error_backtrace = true;
                };

                eprintln!("        Cause {}\n", backtrace);
            };
        });
    }

    if !duplicate_error_backtrace && !error_backtrace.is_empty() {
        eprintln!("        Error {}\n", error_backtrace);
    };
}
