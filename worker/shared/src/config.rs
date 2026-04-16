use anyhow::{anyhow, Context, Result};
use std::{
    env,
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Clone, Debug)]
pub enum DatabaseType {
    Postgres,
    Mysql,
    Sqlite,
}

impl DatabaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::Sqlite => "sqlite",
        }
    }
}

impl Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DatabaseType {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "postgres" => Ok(Self::Postgres),
            "mysql" => Ok(Self::Mysql),
            "sqlite" => Ok(Self::Sqlite),
            other => Err(anyhow!("unsupported DATABASE_TYPE: {other}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub service_name: String,
    pub bind_addr: String,
    pub database_type: DatabaseType,
    pub database_url: String,
    pub job_lock_dir: String,
    pub redis_url: Option<String>,
    pub ready_require_cache: bool,
    pub update_interval_seconds: u64,
    pub source_snapshot_url: Option<String>,
    pub temp_asset_dir: String,
}

impl AppConfig {
    pub fn from_env(service_name: &str) -> Result<Self> {
        load_service_env(service_name)?;

        let database_type = env::var("DATABASE_TYPE")
            .unwrap_or_else(|_| "postgres".to_string())
            .parse::<DatabaseType>()?;

        let database_url = match database_type {
            DatabaseType::Postgres => env::var("POSTGRES_DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres_password@127.0.0.1:3215/station_db".to_string()
            }),
            DatabaseType::Mysql => env::var("MYSQL_DATABASE_URL").unwrap_or_else(|_| {
                "mysql://station_user:station_password@127.0.0.1:3214/station_db".to_string()
            }),
            DatabaseType::Sqlite => env::var("SQLITE_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://storage/sqlite/stations.sqlite3".to_string()),
        };

        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3212".to_string());
        let job_lock_dir = env::var("JOB_LOCK_DIR").unwrap_or_else(|_| "storage/locks".to_string());
        let redis_url = env::var("REDIS_URL").ok().filter(|v| !v.is_empty());
        let ready_require_cache = env::var("READY_REQUIRE_CACHE")
            .unwrap_or_else(|_| "false".to_string())
            .eq_ignore_ascii_case("true");
        let update_interval_seconds = env::var("UPDATE_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(86400);

        let source_snapshot_url = env::var("SOURCE_SNAPSHOT_URL")
            .ok()
            .filter(|v| !v.is_empty());
        let temp_asset_dir =
            env::var("TEMP_ASSET_DIR").unwrap_or_else(|_| "worker/crawler/temp_assets".to_string());

        Ok(Self {
            service_name: service_name.to_string(),
            bind_addr,
            database_type,
            database_url,
            job_lock_dir,
            redis_url,
            ready_require_cache,
            update_interval_seconds,
            source_snapshot_url,
            temp_asset_dir,
        })
    }
}

fn load_service_env(service_name: &str) -> Result<()> {
    let env_path = match service_name {
        "station-api" => Some(Path::new("worker/api/.env")),
        "station-crawler" => Some(Path::new("worker/crawler/.env")),
        "station-ops" => Some(Path::new("worker/ops/.env")),
        _ => None,
    };

    let Some(env_path) = env_path else {
        return Ok(());
    };

    let Some(env_path) = resolve_env_path(env_path) else {
        return Ok(());
    };

    dotenvy::from_path(&env_path)
        .with_context(|| format!("failed to load environment file {}", env_path.display()))?;

    Ok(())
}

fn resolve_env_path(env_path: &Path) -> Option<PathBuf> {
    let roots = build_env_search_roots(env::current_dir().ok(), env::current_exe().ok());

    find_env_path(env_path, &roots)
}

fn build_env_search_roots(
    current_dir: Option<PathBuf>,
    current_exe: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(current_exe) = current_exe {
        if let Some(parent) = current_exe.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    if let Some(current_dir) = current_dir {
        if !roots.iter().any(|root| root == &current_dir) {
            roots.push(current_dir);
        }
    }

    roots
}

fn find_env_path(env_path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    for root in roots {
        for ancestor in root.ancestors() {
            let candidate = ancestor.join(env_path);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn finds_env_file_from_ancestor_root() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("station-config-env-{unique}"));
        let nested = root.join("worker/ops");
        fs::create_dir_all(&nested)?;
        fs::write(nested.join(".env"), "DATABASE_TYPE=postgres\n")?;

        let resolved = find_env_path(Path::new("worker/ops/.env"), &[nested.clone()])
            .expect("env file should resolve from ancestor");

        assert_eq!(resolved, nested.join(".env"));

        fs::remove_dir_all(root)?;

        Ok(())
    }

    #[test]
    fn prefers_executable_root_over_current_dir() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let base = std::env::temp_dir().join(format!("station-config-order-{unique}"));
        let cwd_root = base.join("cwd-root");
        let exe_root = base.join("exe-root");
        let cwd_env = cwd_root.join("worker/ops/.env");
        let exe_env = exe_root.join("worker/ops/.env");

        fs::create_dir_all(cwd_env.parent().expect("cwd env parent"))?;
        fs::create_dir_all(exe_env.parent().expect("exe env parent"))?;
        fs::write(&cwd_env, "DATABASE_TYPE=mysql\n")?;
        fs::write(&exe_env, "DATABASE_TYPE=postgres\n")?;

        let roots = build_env_search_roots(
            Some(cwd_root.clone()),
            Some(exe_root.join("target/release/station-ops")),
        );
        let resolved = find_env_path(Path::new("worker/ops/.env"), &roots)
            .expect("env file should resolve from executable root first");

        assert_eq!(resolved, exe_env);

        fs::remove_dir_all(base)?;

        Ok(())
    }
}
