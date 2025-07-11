use clap::Parser;
use eyre::Context;
use futures::{StreamExt, stream::FuturesUnordered};
use sky_orm_sqlparse::{
    db::{DbType, get_database_url},
    query::parse_tables,
    schema::SqlSchema,
};
use sqlx::Row;
use sqlx::{Connection, SqliteConnection};
use tracing::info;

/// (Re-)Generate the database schema in JSON format
#[derive(Parser, Debug)]
pub struct GenerateSchema {
    /// The URL to the database for which to generate the schema. If left unset, will be pulled
    /// from the `DATABASE_URL` environment variable, or a corresponding `.env` file instead.
    #[arg(short, long, value_name = "DATABASE_URL")]
    database_url: Option<String>,
}

impl GenerateSchema {
    pub async fn run(&self) -> eyre::Result<()> {
        let Some(database_url) = self.database_url.clone().or_else(get_database_url) else {
            return Err(eyre::eyre!(
                "Missing database URL, either set the `DATABASE_URL` environment variable, or specify it manually via --database-url [URL]"
            ));
        };

        let Some(database_type) = DbType::from_connection_string(&database_url) else {
            return Err(eyre::eyre!(
                "Failed to determine database type from connection string, ensure it starts with either `postgres`, `mysql`, or `sqlite`."
            ));
        };

        let schema = match database_type {
            DbType::MySql => todo!(),
            DbType::Postgres => todo!(),
            DbType::Sqlite => generate_sqlite_schema(&database_url).await,
        }?;

        let sky_orm_dir = std::env::current_dir()
            .context("Failed to determine current directory")?
            .join("sky_orm");

        let schema_file = sky_orm_dir.join("schema.json");

        tokio::fs::create_dir_all(&sky_orm_dir)
            .await
            .context("Failed to create sky_orm directory")?;

        tokio::fs::write(
            &schema_file,
            serde_json::to_string_pretty(&schema).context("Failed to serialize schema")?,
        )
        .await
        .context("Failed to write schema")?;

        info!(
            "Schema file updated under {}",
            schema_file.to_str().unwrap()
        );

        Ok(())
    }
}

pub async fn generate_sqlite_schema(url: &str) -> eyre::Result<SqlSchema> {
    let mut conn = SqliteConnection::connect(url)
        .await
        .context("Failed to connect to database")?;

    let tables = sqlx::query("SELECT type,sql FROM sqlite_schema")
        .fetch(&mut conn)
        .filter_map(async |e| match e {
            Ok(e) => {
                let ty: String = e.get("type");
                if ty.eq("table") {
                    let sql: String = e.get("sql");

                    let table = parse_tables(&sql);

                    match table {
                        Ok(t) => Some(Ok(t)),
                        Err(e) => Some(Err(eyre::eyre!("Failed to parse table SQL: {e}"))),
                    }
                } else {
                    None
                }
            }
            Err(e) => Some(Err(eyre::eyre!("Failed to execute DB query: {e}"))),
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to gather tables")?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    Ok(SqlSchema { tables })
}
