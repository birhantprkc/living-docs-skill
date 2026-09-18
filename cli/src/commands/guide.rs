//! `guide` verb wrapper (renamed from `skill` by ADR 0060): resolves the
//! guide's target skill/topic and its plain-text-vs-JSON output mode, then
//! delegates to the embedded skill corpus.

use crate::args::GuideArgs;
use crate::skill;
use crate::store::report_failure;
use std::io::IsTerminal;
use std::process::ExitCode;

const DEFAULT_SKILL: &str = "living-docs";

/// The resolved output shape for `guide`'s success path (ADR 0014). Errors
/// are unaffected — they always print plain text to stderr regardless of
/// mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Plain,
    Json,
}

fn resolve_guide_output(json: bool, plain: bool, is_tty: bool) -> OutputMode {
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

/// The skill and, optionally, the topic `guide` resolves to run against.
struct Target {
    skill: String,
    topic: Option<String>,
}

/// Resolves [`GuideArgs`] to a [`Target`] (see the struct's own docblock for
/// the shapes this reconciles):
/// 1. The hidden `--topic` flag (the retired `skill <name> --topic <t>`
///    alias shape) wins outright: `--skill`, or else the positional, names
///    the skill; `--topic` is the topic.
/// 2. Otherwise, `--skill` given explicitly names the skill; the positional,
///    if any, is the topic.
/// 3. Otherwise, a positional that names a known embedded skill is the
///    skill, with no topic — prints that skill's full `SKILL.md` body.
/// 4. Otherwise, the default skill (`living-docs`); the positional, if any,
///    is the topic.
fn resolve_target(args: &GuideArgs) -> Target {
    if let Some(topic) = &args.topic {
        let skill = args
            .skill
            .clone()
            .or_else(|| args.topic_or_skill.clone())
            .unwrap_or_else(|| DEFAULT_SKILL.to_owned());
        return Target {
            skill,
            topic: Some(topic.clone()),
        };
    }
    if let Some(skill) = &args.skill {
        return Target {
            skill: skill.clone(),
            topic: args.topic_or_skill.clone(),
        };
    }
    if let Some(candidate) = &args.topic_or_skill {
        if skill::skill_names().contains(candidate) {
            return Target {
                skill: candidate.clone(),
                topic: None,
            };
        }
    }
    Target {
        skill: DEFAULT_SKILL.to_owned(),
        topic: args.topic_or_skill.clone(),
    }
}

pub(crate) fn run_guide(args: GuideArgs) -> ExitCode {
    let mode = resolve_guide_output(args.json, args.plain, std::io::stdout().is_terminal());
    let as_json = mode == OutputMode::Json;
    if args.list {
        return print_guide_result(if as_json {
            skill::list_json()
        } else {
            skill::list()
        });
    }
    let target = resolve_target(&args);
    match target.topic {
        Some(topic) => print_guide_result(if as_json {
            skill::topic_json(&target.skill, &topic)
        } else {
            skill::topic(&target.skill, &topic)
        }),
        None => print_guide_result(if as_json {
            skill::body_json(&target.skill)
        } else {
            skill::body(&target.skill)
        }),
    }
}

fn print_guide_result(result: Result<String, String>) -> ExitCode {
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

    fn args(topic_or_skill: Option<&str>, skill: Option<&str>, topic: Option<&str>) -> GuideArgs {
        GuideArgs {
            topic_or_skill: topic_or_skill.map(str::to_owned),
            skill: skill.map(str::to_owned),
            topic: topic.map(str::to_owned),
            list: false,
            json: false,
            plain: false,
        }
    }

    #[test]
    fn bare_positional_topic_resolves_under_the_default_skill() {
        let target = resolve_target(&args(Some("adr"), None, None));
        assert_eq!(target.skill, "living-docs");
        assert_eq!(target.topic.as_deref(), Some("adr"));
    }

    #[test]
    fn positional_matching_a_known_skill_selects_it_with_no_topic() {
        let target = resolve_target(&args(Some("okf-knowledge-format"), None, None));
        assert_eq!(target.skill, "okf-knowledge-format");
        assert_eq!(target.topic, None);
    }

    #[test]
    fn no_positional_resolves_to_the_default_skill_with_no_topic() {
        let target = resolve_target(&args(None, None, None));
        assert_eq!(target.skill, "living-docs");
        assert_eq!(target.topic, None);
    }

    #[test]
    fn explicit_skill_flag_treats_the_positional_as_a_topic() {
        let target = resolve_target(&args(
            Some("conformance"),
            Some("okf-knowledge-format"),
            None,
        ));
        assert_eq!(target.skill, "okf-knowledge-format");
        assert_eq!(target.topic.as_deref(), Some("conformance"));
    }

    #[test]
    fn retired_alias_shape_name_then_topic_flag_still_resolves() {
        let target = resolve_target(&args(Some("living-docs"), None, Some("adr")));
        assert_eq!(target.skill, "living-docs");
        assert_eq!(target.topic.as_deref(), Some("adr"));
    }

    #[test]
    fn resolve_guide_output_json_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_guide_output(true, false, true), OutputMode::Json);
        assert_eq!(resolve_guide_output(true, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_guide_output_plain_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_guide_output(false, true, true), OutputMode::Plain);
        assert_eq!(resolve_guide_output(false, true, false), OutputMode::Plain);
    }

    #[test]
    fn resolve_guide_output_defaults_to_json_when_stdout_is_not_a_tty() {
        assert_eq!(resolve_guide_output(false, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_guide_output_defaults_to_plain_when_stdout_is_a_tty() {
        assert_eq!(resolve_guide_output(false, false, true), OutputMode::Plain);
    }
}
