//! Output-mode and color resolution shared by every data verb (ADR 0060):
//! JSON off a TTY, colored plain text on one, honoring `--json`/`--plain`,
//! `--color`/`NO_COLOR`, and `--quiet` for informational stderr lines.

use clap::ValueEnum;
use std::io::IsTerminal;

/// Whether a data verb's success output is minified JSON or human text.
/// `--json`/`--plain` are explicit overrides that always win; with neither
/// given, a TTY defaults to text and anything else (piped, an agent) to
/// JSON. Errors are unaffected by this mode — they always print plain text
/// to stderr.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Text,
    Json,
}

impl OutputMode {
    /// `color_always` is an explicit `--color=always`, which implies text
    /// mode the same way `--plain` does — a piped `check --color=always`
    /// prints colored text, not JSON (ADR 0060).
    pub(crate) fn resolve(
        json: bool,
        plain: bool,
        color_always: bool,
        stdout_is_tty: bool,
    ) -> Self {
        if json {
            return Self::Json;
        }
        if plain || color_always || stdout_is_tty {
            Self::Text
        } else {
            Self::Json
        }
    }

    pub(crate) fn from_flags(json: bool, plain: bool, color: ColorChoice) -> Self {
        Self::resolve(
            json,
            plain,
            color == ColorChoice::Always,
            std::io::stdout().is_terminal(),
        )
    }

    pub(crate) fn is_json(self) -> bool {
        self == Self::Json
    }
}

/// The `--color` flag's vocabulary (clig.dev convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// Whether ANSI color escapes are emitted. `Auto` follows the TTY and
/// `NO_COLOR` (any non-empty value disables it, per the NO_COLOR convention);
/// `Always`/`Never` override both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColorMode {
    On,
    Off,
}

impl ColorMode {
    pub(crate) fn resolve(choice: ColorChoice, no_color_env: bool, stdout_is_tty: bool) -> Self {
        match choice {
            ColorChoice::Always => Self::On,
            ColorChoice::Never => Self::Off,
            ColorChoice::Auto if no_color_env || !stdout_is_tty => Self::Off,
            ColorChoice::Auto => Self::On,
        }
    }

    pub(crate) fn from_choice(choice: ColorChoice) -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        Self::resolve(choice, no_color, std::io::stdout().is_terminal())
    }
}

/// The palette `paint` supports — just enough for `check`'s report (red
/// violations, yellow advisories, a green or red verdict line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Red,
    Yellow,
    Green,
}

/// Wraps `text` in `style`'s ANSI escape under [`ColorMode::On`]; returns
/// `text` unchanged under [`ColorMode::Off`]. Plain ANSI, no color crate.
pub(crate) fn paint(mode: ColorMode, style: Style, text: &str) -> String {
    if mode == ColorMode::Off {
        return text.to_owned();
    }
    let code = match style {
        Style::Red => "31",
        Style::Yellow => "33",
        Style::Green => "32",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Serializes `value` to minified JSON, matching every embedded-corpus JSON
/// helper's shape. Falls back to a JSON error envelope on a serialization
/// bug rather than panicking — `value`'s fields are always plain owned data,
/// so this only fires if a future field stops being serializable.
pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|err| format!("{{\"error\":\"failed to serialize JSON: {err}\"}}"))
}

/// Prints an informational line to stderr unless `--quiet` was given —
/// warnings and progress notes, never errors (which always print).
pub(crate) fn note(quiet: bool, message: &str) {
    if !quiet {
        eprintln!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_mode_json_flag_wins_regardless_of_tty() {
        assert_eq!(
            OutputMode::resolve(true, false, false, true),
            OutputMode::Json
        );
        assert_eq!(
            OutputMode::resolve(true, false, false, false),
            OutputMode::Json
        );
    }

    #[test]
    fn output_mode_plain_flag_wins_regardless_of_tty() {
        assert_eq!(
            OutputMode::resolve(false, true, false, true),
            OutputMode::Text
        );
        assert_eq!(
            OutputMode::resolve(false, true, false, false),
            OutputMode::Text
        );
    }

    #[test]
    fn output_mode_defaults_to_json_off_a_tty() {
        assert_eq!(
            OutputMode::resolve(false, false, false, false),
            OutputMode::Json
        );
    }

    #[test]
    fn output_mode_defaults_to_text_on_a_tty() {
        assert_eq!(
            OutputMode::resolve(false, false, false, true),
            OutputMode::Text
        );
    }

    #[test]
    fn output_mode_explicit_color_always_implies_text_off_a_tty() {
        assert_eq!(
            OutputMode::resolve(false, false, true, false),
            OutputMode::Text
        );
    }

    #[test]
    fn output_mode_json_flag_wins_over_explicit_color_always() {
        assert_eq!(
            OutputMode::resolve(true, false, true, false),
            OutputMode::Json
        );
    }

    #[test]
    fn color_mode_always_wins_off_a_tty() {
        assert_eq!(
            ColorMode::resolve(ColorChoice::Always, false, false),
            ColorMode::On
        );
    }

    #[test]
    fn color_mode_never_wins_on_a_tty() {
        assert_eq!(
            ColorMode::resolve(ColorChoice::Never, false, true),
            ColorMode::Off
        );
    }

    #[test]
    fn color_mode_auto_is_off_when_no_color_env_is_set_even_on_a_tty() {
        assert_eq!(
            ColorMode::resolve(ColorChoice::Auto, true, true),
            ColorMode::Off
        );
    }

    #[test]
    fn color_mode_auto_is_off_when_stdout_is_not_a_tty() {
        assert_eq!(
            ColorMode::resolve(ColorChoice::Auto, false, false),
            ColorMode::Off
        );
    }

    #[test]
    fn color_mode_auto_is_on_for_a_plain_tty() {
        assert_eq!(
            ColorMode::resolve(ColorChoice::Auto, false, true),
            ColorMode::On
        );
    }

    #[test]
    fn paint_wraps_text_in_the_styles_ansi_escape_when_on() {
        let painted = paint(ColorMode::On, Style::Red, "boom");
        assert_eq!(painted, "\x1b[31mboom\x1b[0m");
    }

    #[test]
    fn paint_returns_the_text_unchanged_when_off() {
        assert_eq!(paint(ColorMode::Off, Style::Green, "ok"), "ok");
    }
}
