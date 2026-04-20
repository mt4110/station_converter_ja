use std::sync::Once;

use anyhow::Result;
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
        Err(_) => Ok(String::from_utf8(decode_bytes(row, column)?)?),
    }
}

pub fn decode_optional_string(row: &AnyRow, column: &str) -> Result<Option<String>> {
    match row.try_get::<Option<String>, _>(column) {
        Ok(value) => Ok(value),
        Err(_) => row
            .try_get::<Option<Vec<u8>>, _>(column)
            .map_err(anyhow::Error::from)?
            .map(String::from_utf8)
            .transpose()
            .map_err(Into::into),
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

fn decode_bytes(row: &AnyRow, column: &str) -> std::result::Result<Vec<u8>, sqlx::Error> {
    row.try_get::<Vec<u8>, _>(column)
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
