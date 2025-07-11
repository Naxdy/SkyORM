use sqlx::Database;

/// Trait describing a struct that may be parsed from a [`sqlx::Row`].
pub trait ParseFromRow<DB>: Sized
where
    DB: Database,
{
    /// Attempt to parse this struct from a database row.
    ///
    /// # Errors
    ///
    /// On parse failure. See [`sqlx::Error`] for more information.
    fn parse_from_row(row: &<DB as Database>::Row) -> Result<Self, sqlx::Error>;
}
