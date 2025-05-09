use sqlx::any::AnyRow;

/// Trait describing a struct that may be parsed from a [`sqlx::Row`].
pub trait ParseFromRow: Sized {
    fn parse_from_row(row: &AnyRow) -> Result<Self, sqlx::Error>;
}
