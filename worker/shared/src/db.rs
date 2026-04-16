use std::sync::Once;

use anyhow::Result;
use sqlx::{any::AnyPoolOptions, AnyPool};

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
