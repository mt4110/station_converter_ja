use std::sync::Once;

use anyhow::{Context, Result};
use sqlx::{
    any::{AnyPoolOptions, AnyRow},
    AnyPool, Row,
};

use crate::config::DatabaseType;

static INSTALL_DRIVERS: Once = Once::new();

#[derive(Clone, Copy, Debug)]
pub enum SqlDialect {
    Postgres,
    Mysql,
    Sqlite,
}

impl SqlDialect {
    pub fn statement(self, sql: &str) -> String {
        if !matches!(self, Self::Postgres) {
            return sql.to_string();
        }

        let mut parameter_index = 1usize;
        let mut output = String::with_capacity(sql.len() + 16);

        for character in sql.chars() {
            if character == '?' {
                output.push('$');
                output.push_str(&parameter_index.to_string());
                parameter_index += 1;
            } else {
                output.push(character);
            }
        }

        output
    }

    pub fn timestamp_parameter(self) -> &'static str {
        match self {
            Self::Postgres => "CAST(? AS TIMESTAMPTZ)",
            Self::Mysql | Self::Sqlite => "?",
        }
    }

    pub fn text_cast(self, expression: &str) -> String {
        match self {
            Self::Postgres | Self::Sqlite => format!("CAST({expression} AS TEXT)"),
            Self::Mysql => format!("CAST({expression} AS CHAR)"),
        }
    }
}

impl From<&DatabaseType> for SqlDialect {
    fn from(value: &DatabaseType) -> Self {
        match value {
            DatabaseType::Postgres => Self::Postgres,
            DatabaseType::Mysql => Self::Mysql,
            DatabaseType::Sqlite => Self::Sqlite,
        }
    }
}

pub fn decode_required_string(row: &AnyRow, column: &str) -> Result<String> {
    match row.try_get::<String, _>(column) {
        Ok(value) => Ok(value),
        Err(err) if should_retry_string_decode_as_bytes(&err) => {
            String::from_utf8(decode_bytes(row, column)?)
                .with_context(|| format!("column '{column}' contained non-utf8 bytes"))
        }
        Err(err) => Err(err.into()),
    }
}

pub fn decode_optional_string(row: &AnyRow, column: &str) -> Result<Option<String>> {
    match row.try_get::<Option<String>, _>(column) {
        Ok(value) => Ok(value),
        Err(err) if should_retry_string_decode_as_bytes(&err) => row
            .try_get::<Option<Vec<u8>>, _>(column)
            .map_err(anyhow::Error::from)?
            .map(|bytes| {
                String::from_utf8(bytes)
                    .with_context(|| format!("column '{column}' contained non-utf8 bytes"))
            })
            .transpose(),
        Err(err) => Err(err.into()),
    }
}

pub fn integer_aggregate_sql(dialect: SqlDialect, expr: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("CAST(COALESCE({expr}, 0) AS SIGNED)"),
        SqlDialect::Postgres | SqlDialect::Sqlite => {
            format!("CAST(COALESCE({expr}, 0) AS BIGINT)")
        }
    }
}

pub fn distinct_text_count_sql(dialect: SqlDialect, column: &str) -> String {
    match dialect {
        SqlDialect::Mysql => format!("COUNT(DISTINCT CAST({column} AS BINARY))"),
        SqlDialect::Postgres | SqlDialect::Sqlite => format!("COUNT(DISTINCT {column})"),
    }
}

pub fn prefix_scope_sql(dialect: SqlDialect, column: &str, prefix_len: usize) -> String {
    match dialect {
        SqlDialect::Mysql => format!("{column} COLLATE utf8mb4_bin LIKE ? ESCAPE '\\\\'"),
        SqlDialect::Postgres | SqlDialect::Sqlite => {
            format!("substr({column}, 1, {prefix_len}) = ?")
        }
    }
}

pub fn prefix_scope_arg(dialect: SqlDialect, prefix: &str) -> String {
    match dialect {
        SqlDialect::Mysql => like_prefix_pattern(prefix),
        SqlDialect::Postgres | SqlDialect::Sqlite => prefix.to_string(),
    }
}

pub fn like_prefix_pattern(prefix: &str) -> String {
    let escaped = prefix
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}%")
}

fn decode_bytes(row: &AnyRow, column: &str) -> std::result::Result<Vec<u8>, sqlx::Error> {
    row.try_get::<Vec<u8>, _>(column)
}

fn should_retry_string_decode_as_bytes(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Decode(_) | sqlx::Error::ColumnDecode { .. }
    )
}

pub fn ensure_sqlx_drivers() {
    INSTALL_DRIVERS.call_once(sqlx::any::install_default_drivers);
}

pub async fn connect_any_pool(database_url: &str) -> Result<AnyPool> {
    ensure_sqlx_drivers();

    Ok(AnyPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::should_retry_string_decode_as_bytes;

    #[test]
    fn retries_only_for_decode_errors() {
        assert!(should_retry_string_decode_as_bytes(&sqlx::Error::Decode(
            "decode".into()
        )));
        assert!(should_retry_string_decode_as_bytes(
            &sqlx::Error::ColumnDecode {
                index: "station_uid".to_string(),
                source: "decode".into(),
            }
        ));
        assert!(!should_retry_string_decode_as_bytes(
            &sqlx::Error::ColumnNotFound("station_uid".to_string())
        ));
    }
}
