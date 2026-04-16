use std::{
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct JobLockBusy {
    lock_name: String,
    path: PathBuf,
    holder_summary: Option<String>,
}

impl fmt::Display for JobLockBusy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "job lock '{}' is already held at {}",
            self.lock_name,
            self.path.display()
        )?;

        if let Some(holder_summary) = &self.holder_summary {
            write!(f, " ({holder_summary})")?;
        }

        Ok(())
    }
}

impl Error for JobLockBusy {}

#[derive(Debug)]
pub struct JobLockGuard {
    file: File,
}

impl Drop for JobLockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LockHolder {
    acquired_at: String,
    pid: u32,
    service_name: String,
}

pub fn try_acquire_job_lock(
    lock_dir: impl AsRef<Path>,
    lock_name: &str,
    service_name: &str,
) -> Result<JobLockGuard> {
    validate_lock_name(lock_name)?;

    let lock_dir = lock_dir.as_ref();
    fs::create_dir_all(lock_dir)
        .with_context(|| format!("failed to create lock directory {}", lock_dir.display()))?;

    let path = lock_dir.join(format!("{lock_name}.lock"));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("failed to open job lock file {}", path.display()))?;

    if let Err(err) = file.try_lock_exclusive() {
        if err.kind() == std::io::ErrorKind::WouldBlock {
            let holder_summary = read_holder_summary(&mut file).ok().flatten();
            return Err(JobLockBusy {
                lock_name: lock_name.to_string(),
                path,
                holder_summary,
            }
            .into());
        }

        return Err(err).with_context(|| format!("failed to acquire job lock {}", path.display()));
    }

    write_holder(
        &mut file,
        &LockHolder {
            acquired_at: Utc::now().to_rfc3339(),
            pid: std::process::id(),
            service_name: service_name.to_string(),
        },
    )?;

    Ok(JobLockGuard { file })
}

fn validate_lock_name(lock_name: &str) -> Result<()> {
    if lock_name.is_empty() {
        bail!("job lock name must not be empty");
    }

    if !lock_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("invalid job lock name '{lock_name}': use only ASCII letters, digits, '-' or '_'");
    }

    Ok(())
}

fn read_holder_summary(file: &mut File) -> Result<Option<String>> {
    file.seek(SeekFrom::Start(0))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let summary = match serde_json::from_str::<LockHolder>(trimmed) {
        Ok(holder) => format!(
            "held by {} (pid {}, acquired {})",
            holder.service_name, holder.pid, holder.acquired_at
        ),
        Err(_) => trimmed.to_string(),
    };

    Ok(Some(summary))
}

fn write_holder(file: &mut File, holder: &LockHolder) -> Result<()> {
    let encoded = serde_json::to_vec(holder)?;

    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_data()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::Result;

    use super::{try_acquire_job_lock, JobLockBusy};

    #[test]
    fn job_lock_blocks_second_holder_until_release() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = std::env::temp_dir().join(format!("station-job-lock-test-{unique}"));

        let first_guard = try_acquire_job_lock(&dir, "ingest-n02", "first")?;
        let second_attempt = try_acquire_job_lock(&dir, "ingest-n02", "second")
            .expect_err("second lock acquisition should fail");
        assert!(second_attempt.downcast_ref::<JobLockBusy>().is_some());

        drop(first_guard);

        let second_guard = try_acquire_job_lock(&dir, "ingest-n02", "second")?;
        drop(second_guard);
        fs::remove_dir_all(&dir)?;

        Ok(())
    }

    #[test]
    fn rejects_lock_name_with_path_separators() {
        let err = try_acquire_job_lock(std::env::temp_dir(), "../ingest-n02", "first")
            .expect_err("path traversal lock name should be rejected");

        assert!(
            err.to_string().contains("invalid job lock name"),
            "unexpected error: {err}"
        );
    }
}
