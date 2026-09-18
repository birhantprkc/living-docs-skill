//! One module per CLI subcommand wrapper, mirroring `living_docs_core::commands` (issue 0028).

pub(crate) mod check;
pub(crate) mod effective;
pub(crate) mod export;
pub(crate) mod fmt;
pub(crate) mod hooks_cmd;
pub(crate) mod index;
pub(crate) mod leak_gate;
pub(crate) mod migrate;
pub(crate) mod new;
pub(crate) mod set;
pub(crate) mod skill_cmd;
pub(crate) mod supersede;
