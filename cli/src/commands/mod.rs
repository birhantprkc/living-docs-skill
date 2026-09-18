//! One module per CLI subcommand wrapper, mirroring `living_docs_core::commands` (issue 0028).

pub(crate) mod check;
pub(crate) mod fmt;
pub(crate) mod guide;
pub(crate) mod index;
pub(crate) mod install;
pub(crate) mod new;
pub(crate) mod read;
pub(crate) mod set;
pub(crate) mod supersede;
pub(crate) mod uninstall;
