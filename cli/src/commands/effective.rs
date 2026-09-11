//! `effective` verb wrapper: builds the active-backend store, then delegates
//! to `living_docs_core::commands::effective::run` to compile and print the
//! agent-facing effective view.

use crate::args::{EffectiveArgs, TierArg};
use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands::effective::{self, Options, Tier};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_effective(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    args: EffectiveArgs,
) -> ExitCode {
    let options = Options {
        topic: args.topic,
        tier: tier_of(args.tier),
        budget: args.budget,
        include_stale: args.include_stale,
    };
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => effective::run(store.as_ref(), docs_dir, &options),
        Err(err) => report_failure(&err),
    }
}

fn tier_of(tier: TierArg) -> Tier {
    match tier {
        TierArg::Index => Tier::Index,
        TierArg::Outline => Tier::Outline,
        TierArg::Full => Tier::Full,
    }
}
