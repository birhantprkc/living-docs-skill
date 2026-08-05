//! Database engine and persistence backend selection: clap enums plus connection-string resolution.

use clap::ValueEnum;

pub(crate) const SQLITE_READ_MODEL_PATH: &str = ".living-docs/index.db";
const DATABASE_URL_VAR: &str = "DATABASE_URL";

/// The database backend to connect to, selectable via the global `--engine`
/// flag (ADR 0004, issue 0004). `Paradedb` is the default, requiring
/// `$DATABASE_URL`; `Sqlite` is opt-in and falls back to the local embedded
/// read-model when `$DATABASE_URL` is unset.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Engine {
    Sqlite,
    Paradedb,
}

/// The persistence backend `new`/`check`/`export` operate against (ADR
/// 0007, issue 0006 slice 0006-D2).
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Backend {
    Fs,
    Db,
}

impl Engine {
    pub(crate) fn resolve_url(self) -> Result<String, String> {
        self.resolve_url_with(|name| std::env::var(name))
    }

    /// `Sqlite` honors `$DATABASE_URL` when set (accepting a full
    /// `sqlite://…` value, e.g. a hermetic per-test database), falling back
    /// to the local read-model path otherwise; `Paradedb` requires
    /// `$DATABASE_URL` unconditionally.
    fn resolve_url_with(
        self,
        lookup_env: impl Fn(&str) -> Result<String, std::env::VarError>,
    ) -> Result<String, String> {
        match self {
            Engine::Sqlite => {
                Ok(lookup_env(DATABASE_URL_VAR).unwrap_or_else(|_| default_sqlite_url()))
            }
            Engine::Paradedb => lookup_env(DATABASE_URL_VAR).map_err(|_| {
                format!(
                    "the paradedb engine requires ${DATABASE_URL_VAR} to be set to a Postgres connection string"
                )
            }),
        }
    }
}

/// The connection string `Engine::Sqlite` resolves to when `$DATABASE_URL`
/// is unset — the single source of truth for what "the default local
/// SQLite backend" means, shared by [`Engine::resolve_url_with`] and
/// [`is_default_local_sqlite`].
pub(crate) fn default_sqlite_url() -> String {
    format!("sqlite://{SQLITE_READ_MODEL_PATH}?mode=rwc")
}

/// True only when `engine`/`url` is the default local SQLite backend
/// (`Engine::Sqlite` with `$DATABASE_URL` unset), the one case where the
/// `.living-docs/index.db` file existence check in [`crate::run_search`] is
/// a reliable signal — a `Sqlite` engine pointed at an overridden URL, or
/// `Paradedb`, may have no local file at all yet still have a valid index.
pub(crate) fn is_default_local_sqlite(engine: Engine, url: &str) -> bool {
    matches!(engine, Engine::Sqlite) && url == default_sqlite_url()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_sqlite_resolves_to_the_local_read_model_url_when_database_url_is_unset() {
        let url = Engine::Sqlite
            .resolve_url_with(|_| Err(std::env::VarError::NotPresent))
            .expect("sqlite url always resolves");
        assert_eq!(url, format!("sqlite://{SQLITE_READ_MODEL_PATH}?mode=rwc"));
    }

    #[test]
    fn engine_sqlite_honors_database_url_when_set() {
        let url = Engine::Sqlite
            .resolve_url_with(|_| Ok("sqlite:///tmp/hermetic.db?mode=rwc".to_owned()))
            .expect("sqlite url resolves from the override");
        assert_eq!(url, "sqlite:///tmp/hermetic.db?mode=rwc");
    }

    #[test]
    fn engine_paradedb_resolves_the_configured_database_url() {
        let url = Engine::Paradedb
            .resolve_url_with(|_| Ok("postgres://user:pass@localhost/db".to_owned()))
            .expect("paradedb url resolves when DATABASE_URL is set");
        assert_eq!(url, "postgres://user:pass@localhost/db");
    }

    #[test]
    fn engine_paradedb_errors_clearly_when_database_url_is_unset() {
        let err = Engine::Paradedb
            .resolve_url_with(|_| Err(std::env::VarError::NotPresent))
            .expect_err("paradedb url resolution fails without DATABASE_URL");
        assert!(err.contains(DATABASE_URL_VAR), "got: {err}");
    }

    #[test]
    fn is_default_local_sqlite_is_true_for_sqlite_with_the_default_url() {
        assert!(is_default_local_sqlite(
            Engine::Sqlite,
            &default_sqlite_url()
        ));
    }

    #[test]
    fn is_default_local_sqlite_is_false_for_sqlite_with_an_overridden_url() {
        assert!(!is_default_local_sqlite(
            Engine::Sqlite,
            "sqlite:///tmp/hermetic.db?mode=rwc"
        ));
    }

    #[test]
    fn is_default_local_sqlite_is_false_for_paradedb_even_with_the_default_sqlite_url_string() {
        assert!(!is_default_local_sqlite(
            Engine::Paradedb,
            &default_sqlite_url()
        ));
    }
}
