//! `skill` verb wrapper: resolves plain-text-vs-JSON output mode, then delegates to the embedded skill corpus.

use crate::skill;
use crate::store::report_failure;
use std::io::IsTerminal;
use std::process::ExitCode;

/// The resolved output shape for `skill`'s success path (ADR 0014, "output
/// format is context-aware"). Errors are unaffected — they always print
/// plain text to stderr regardless of mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Plain,
    Json,
}

/// Resolves `skill`'s effective [`OutputMode`]: `json`/`plain` are explicit
/// overrides and always win over autodetection, `json` taking precedence if
/// both were somehow set (clap's `conflicts_with` already rejects that
/// combination before this runs). With neither flag given, `is_tty` decides
/// — a TTY (interactive human) defaults to plain text, anything else
/// (piped, an agent consuming the output) defaults to JSON. `is_tty` is a
/// plain parameter, not a live syscall, so this stays unit-testable without
/// a real terminal.
fn resolve_skill_output(json: bool, plain: bool, is_tty: bool) -> OutputMode {
    if json {
        return OutputMode::Json;
    }
    if plain {
        return OutputMode::Plain;
    }
    if is_tty {
        OutputMode::Plain
    } else {
        OutputMode::Json
    }
}

/// `--list` takes priority over `name`/`topic`; otherwise `name` is
/// required and `topic`, when given, narrows the body to one topic's
/// detail (ADR 0014). The resolved [`OutputMode`] swaps every branch's
/// plain-text renderer for its minified-JSON counterpart without changing
/// the selection logic or error handling (errors always stay plain text on
/// stderr).
pub(crate) fn run_skill(
    name: Option<String>,
    topic: Option<String>,
    list: bool,
    json: bool,
    plain: bool,
) -> ExitCode {
    let mode = resolve_skill_output(json, plain, std::io::stdout().is_terminal());
    let as_json = mode == OutputMode::Json;
    if list {
        return print_skill_result(if as_json {
            skill::list_json()
        } else {
            skill::list()
        });
    }
    let Some(name) = name else {
        return report_failure("skill: NAME is required unless --list is given");
    };
    match topic {
        Some(topic) => print_skill_result(if as_json {
            skill::topic_json(&name, &topic)
        } else {
            skill::topic(&name, &topic)
        }),
        None => print_skill_result(if as_json {
            skill::body_json(&name)
        } else {
            skill::body(&name)
        }),
    }
}

fn print_skill_result(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_skill_output_json_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_skill_output(true, false, true), OutputMode::Json);
        assert_eq!(resolve_skill_output(true, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_skill_output_plain_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_skill_output(false, true, true), OutputMode::Plain);
        assert_eq!(resolve_skill_output(false, true, false), OutputMode::Plain);
    }

    #[test]
    fn resolve_skill_output_defaults_to_json_when_stdout_is_not_a_tty() {
        assert_eq!(resolve_skill_output(false, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_skill_output_defaults_to_plain_when_stdout_is_a_tty() {
        assert_eq!(resolve_skill_output(false, false, true), OutputMode::Plain);
    }
}
