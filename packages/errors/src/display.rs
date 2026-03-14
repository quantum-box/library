use crate::Error;

const MAX_BACKTRACE_LINES: usize = 15;

pub(crate) fn format_backtrace(
    backtrace: &std::backtrace::Backtrace,
) -> String {
    let mut output = String::new();
    let backtrace_str = format!("{backtrace}");

    let lines: Vec<&str> = backtrace_str
        .lines()
        .filter(|line| {
            line.contains("at")
                && !line.contains("/rust/")
                && !line.contains("/.cargo/")
                && !line.contains("/rustc/")
        })
        .take(MAX_BACKTRACE_LINES)
        .collect();

    for (i, line) in lines.iter().enumerate() {
        let line = line.trim();
        if !line.is_empty() {
            output.push_str(&format!("    {i}: {line}\n"));
        }
    }

    if backtrace_str.lines().count() > MAX_BACKTRACE_LINES {
        output.push_str("    ... more frames ...\n");
    }

    output
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (message, backtrace) = match self {
            Error::NotFound { message, backtrace }
            | Error::InternalServerError { message, backtrace }
            | Error::BadRequest { message, backtrace }
            | Error::Unauthorized { message, backtrace }
            | Error::Forbidden { message, backtrace }
            | Error::Conflict { message, backtrace }
            | Error::PaymentRequired { message, backtrace }
            | Error::ServiceUnavailable { message, backtrace } => {
                (message, backtrace)
            }
        };

        write!(f, "{message}")?;

        if f.alternate() {
            write!(f, "\n\nBacktrace:\n{}", format_backtrace(backtrace))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn create_test_error() -> Error {
        Error::business_logic("test error message")
    }

    #[test]
    fn test_error_display_format() {
        let error = create_test_error();
        let debug_output = format!("{:?}", error);

        assert!(
            debug_output.contains("BusinessLogicError: test error message")
        );
        assert!(!debug_output.contains("Backtrace:"));

        let pretty_debug_output = format!("{:#?}", error);
        assert!(pretty_debug_output.contains("Backtrace:"));

        let lines: Vec<&str> = pretty_debug_output.lines().collect();
        assert!(lines.len() > 2);

        let backtrace_lines: Vec<&str> = pretty_debug_output
            .lines()
            .skip_while(|line| !line.contains("Backtrace:"))
            .skip(1)
            .take_while(|line| !line.contains("... more frames ..."))
            .collect();

        for line in backtrace_lines {
            if !line.is_empty() {
                assert!(line.contains("at"));
                assert!(!line.contains("/rust/"));
                assert!(!line.contains("/.cargo/"));
                assert!(!line.contains("/rustc/"));
            }
        }
    }

    #[test]
    fn test_different_error_types() {
        let errors = vec![
            Error::not_found("not found"),
            Error::parse_error(
                serde_json::from_str::<Value>("invalid json")
                    .unwrap_err()
                    .to_string(),
            ),
            Error::permission_denied("no permission"),
        ];

        for error in errors {
            let debug_output = format!("{:?}", error);
            let pretty_debug_output = format!("{:#?}", error);

            match error {
                Error::NotFound { .. } => {
                    assert!(
                        debug_output.contains("NotFoundError: not found")
                    );
                }
                Error::BadRequest { .. } => {
                    assert!(debug_output.contains("ParseError:"));
                }
                Error::Forbidden { .. } => {
                    assert!(debug_output
                        .contains("PermissionDenied: no permission"));
                }
                _ => unreachable!(),
            }

            assert!(!debug_output.contains("Backtrace:"));
            assert!(pretty_debug_output.contains("Backtrace:"));
            assert!(!pretty_debug_output.contains("/rust/"));
            assert!(!pretty_debug_output.contains("/.cargo/"));
            assert!(!pretty_debug_output.contains("/rustc/"));
        }
    }
}
