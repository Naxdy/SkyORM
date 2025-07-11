use dotenvy::dotenv;

pub enum DbType {
    MySql,
    Postgres,
    Sqlite,
}

impl DbType {
    #[must_use]
    pub fn from_connection_string(input: &str) -> Option<Self> {
        let lower = input.to_lowercase();

        if lower.starts_with("postgres") {
            Some(Self::Postgres)
        } else if lower.starts_with("sqlite") {
            Some(Self::Sqlite)
        } else if lower.starts_with("mysql") {
            Some(Self::MySql)
        } else {
            None
        }
    }
}

/// Attempt to retrieve the database URL from the `DATABASE_URL` environment variable, or from a
/// corresponding `.env` file.
///
/// # Panics
///
/// If the environment variable cannot be parsed to UTF8.
#[must_use]
#[allow(clippy::unwrap_used)]
pub fn get_database_url() -> Option<String> {
    let _ = dotenv();

    std::env::var_os("DATABASE_URL").map(|e| e.to_str().unwrap().to_string())
}
