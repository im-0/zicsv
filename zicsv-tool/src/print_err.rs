use failure;

pub fn print_error(error: &failure::Error) {
    error!("Error:");

    let error_backtrace = format!("{}", error.backtrace());
    let mut duplicate_error_backtrace = false;
    for cause in error.causes() {
        error!("    {}", cause);
        let _ = cause.backtrace().map(|backtrace| {
            let backtrace = format!("{}", backtrace);
            if !backtrace.is_empty() {
                if backtrace == error_backtrace {
                    duplicate_error_backtrace = true;
                };

                error!("        Cause {}\n", backtrace);
            };
        });
    }

    if !duplicate_error_backtrace && !error_backtrace.is_empty() {
        error!("        Error {}\n", error_backtrace);
    };
}
