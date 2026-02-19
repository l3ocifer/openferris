use ferris_common::{unix_timestamp, FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

// ── Data types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub schedule: String,
    pub action: String,
    pub enabled: bool,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub result: Option<String>,
}

// ── Action types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskAction {
    Log { message: String },
    Http { url: String, body: Option<String> },
    Webhook { url: String, body: Option<String> },
}

impl TaskAction {
    pub fn parse(action_str: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(action_str)
    }
}

// ── TaskScheduler ──────────────────────────────────────────────────────────

pub struct TaskScheduler {
    pool: SqlitePool,
    max_scheduled: u32,
}

impl TaskScheduler {
    pub fn new(pool: SqlitePool, max_scheduled: u32) -> Self {
        Self { pool, max_scheduled }
    }

    /// Access the underlying connection pool (for status queries).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Schedule a new task (persisted in SQLite).
    ///
    /// `schedule` is a cron expression (e.g., "0 * * * *" for every hour).
    /// `action` is a JSON string describing what to execute (see `TaskAction`).
    pub async fn schedule_task(&self, schedule: &str, action: &str) -> Result<Task> {
        // Validate cron expression
        schedule
            .parse::<croner::Cron>()
            .map_err(|e| FerrisError::InvalidInput(format!("invalid cron expression: {e}")))?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE enabled = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        if count >= self.max_scheduled as i64 {
            return Err(FerrisError::CapacityExceeded(format!(
                "task limit reached ({count}/{})",
                self.max_scheduled
            )));
        }

        let id = Uuid::now_v7().to_string();
        let now = unix_timestamp();

        sqlx::query(
            "INSERT INTO tasks (id, schedule, action, enabled, created_at)
             VALUES (?, ?, ?, 1, ?)",
        )
        .bind(&id)
        .bind(schedule)
        .bind(action)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(Task {
            task_id: id,
            schedule: schedule.into(),
            action: action.into(),
            enabled: true,
            created_at: now,
            last_run_at: None,
        })
    }

    /// List all tasks.
    pub async fn list_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT id, schedule, action, enabled, created_at, last_run_at
             FROM tasks ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    /// Cancel (delete) a task by id.
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(FerrisError::NotFound(format!("task: {task_id}")));
        }
        Ok(())
    }

    /// Get recent runs for a task.
    pub async fn task_runs(&self, task_id: &str, limit: usize) -> Result<Vec<TaskRun>> {
        let rows = sqlx::query(
            "SELECT id, task_id, started_at, completed_at, result
             FROM task_runs WHERE task_id = ?
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(task_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows.iter().map(row_to_run).collect())
    }

    /// Start the background task executor loop. Returns a `JoinHandle` that
    /// can be used to cancel the loop on shutdown.
    pub fn start_executor(
        pool: SqlitePool,
        poll_interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!("Task executor started (poll interval: {poll_interval_secs}s)");
            loop {
                if let Err(e) = execute_due_tasks(&pool).await {
                    tracing::error!("Task executor error: {e}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs)).await;
            }
        })
    }
}

// ── Executor logic ─────────────────────────────────────────────────────────

async fn execute_due_tasks(pool: &SqlitePool) -> Result<()> {
    let now = unix_timestamp();

    let rows = sqlx::query(
        "SELECT id, schedule, action, last_run_at
         FROM tasks WHERE enabled = 1",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| FerrisError::Database(e.to_string()))?;

    for row in &rows {
        let task_id: String = row.get("id");
        let schedule: String = row.get("schedule");
        let action: String = row.get("action");
        let last_run_at: Option<i64> = row.get("last_run_at");

        if is_due(&schedule, last_run_at, now) {
            tracing::debug!("Executing task {task_id}");
            let run_id = Uuid::now_v7().to_string();
            let started_at = unix_timestamp();

            // Record run start
            sqlx::query("INSERT INTO task_runs (id, task_id, started_at) VALUES (?, ?, ?)")
                .bind(&run_id)
                .bind(&task_id)
                .bind(started_at)
                .execute(pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

            let result = execute_action(&action).await;
            let completed_at = unix_timestamp();
            let result_text = match &result {
                Ok(msg) => format!("ok: {msg}"),
                Err(e) => format!("error: {e}"),
            };

            // Record run result
            sqlx::query("UPDATE task_runs SET completed_at = ?, result = ? WHERE id = ?")
                .bind(completed_at)
                .bind(&result_text)
                .bind(&run_id)
                .execute(pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

            // Update last_run_at
            sqlx::query("UPDATE tasks SET last_run_at = ? WHERE id = ?")
                .bind(completed_at)
                .bind(&task_id)
                .execute(pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

            match result {
                Ok(msg) => tracing::info!("Task {task_id} completed: {msg}"),
                Err(e) => tracing::warn!("Task {task_id} failed: {e}"),
            }
        }
    }

    Ok(())
}

/// Check whether a task is due for execution.
fn is_due(schedule: &str, last_run_at: Option<i64>, now_secs: i64) -> bool {
    let cron: croner::Cron = match schedule.parse() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Invalid cron expression '{schedule}': {e}");
            return false;
        }
    };

    let now_dt = chrono_from_unix(now_secs);

    // Find the most recent time the cron should have fired at or before `now`
    let prev_fire = match cron.find_previous_occurrence(&now_dt, true) {
        Ok(dt) => dt.timestamp(),
        Err(_) => return false,
    };

    match last_run_at {
        Some(last_run) => prev_fire > last_run,
        None => true,
    }
}

fn chrono_from_unix(secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default()
}

/// Execute a single task action.
async fn execute_action(action_json: &str) -> std::result::Result<String, String> {
    let action = TaskAction::parse(action_json).map_err(|e| format!("invalid action: {e}"))?;

    match action {
        TaskAction::Log { message } => {
            tracing::info!("[task-action] {message}");
            Ok(message)
        }
        TaskAction::Http { url, body } | TaskAction::Webhook { url, body } => {
            let client = reqwest::Client::new();
            let mut req = client.post(&url);
            if let Some(b) = body {
                req = req.header("content-type", "application/json").body(b);
            }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let status = resp.status();
            if status.is_success() {
                Ok(format!("HTTP {status}"))
            } else {
                let text = resp.text().await.unwrap_or_default();
                Err(format!("HTTP {status}: {text}"))
            }
        }
    }
}

// ── Row mappers ────────────────────────────────────────────────────────────

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Task {
    let enabled: i32 = row.get("enabled");
    Task {
        task_id: row.get("id"),
        schedule: row.get("schedule"),
        action: row.get("action"),
        enabled: enabled != 0,
        created_at: row.get("created_at"),
        last_run_at: row.get("last_run_at"),
    }
}

fn row_to_run(row: &sqlx::sqlite::SqliteRow) -> TaskRun {
    TaskRun {
        id: row.get("id"),
        task_id: row.get("task_id"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        result: row.get("result"),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(":memory:").create_if_missing(true))
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                schedule TEXT NOT NULL,
                action TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                last_run_at INTEGER
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS task_runs (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                result TEXT,
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn schedule_and_list() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 10);

        let action = r#"{"type":"log","message":"hello"}"#;
        let task = scheduler.schedule_task("0 * * * *", action).await.unwrap();
        assert_eq!(task.schedule, "0 * * * *");
        assert_eq!(task.action, action);
        assert!(task.enabled);
        assert!(task.last_run_at.is_none());

        let tasks = scheduler.list_tasks().await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, task.task_id);
    }

    #[tokio::test]
    async fn cancel_removes_task() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 10);

        let action = r#"{"type":"log","message":"test"}"#;
        let task = scheduler.schedule_task("*/5 * * * *", action).await.unwrap();
        scheduler.cancel_task(&task.task_id).await.unwrap();

        let tasks = scheduler.list_tasks().await.unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn cancel_missing_returns_not_found() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 10);

        let err = scheduler.cancel_task("no-such-task").await.unwrap_err();
        assert!(matches!(err, FerrisError::NotFound(_)));
    }

    #[tokio::test]
    async fn capacity_enforcement() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 2);

        let action = r#"{"type":"log","message":"x"}"#;
        scheduler.schedule_task("0 * * * *", action).await.unwrap();
        scheduler.schedule_task("0 * * * *", action).await.unwrap();
        let err = scheduler.schedule_task("0 * * * *", action).await.unwrap_err();
        assert!(matches!(err, FerrisError::CapacityExceeded(_)));
    }

    #[tokio::test]
    async fn invalid_cron_rejected() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 10);

        let action = r#"{"type":"log","message":"x"}"#;
        let err = scheduler.schedule_task("not-a-cron", action).await.unwrap_err();
        assert!(matches!(err, FerrisError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn execute_due_runs_log_action() {
        let pool = test_pool().await;

        // Insert a task that runs every minute, never run before
        let id = Uuid::now_v7().to_string();
        let now = unix_timestamp();
        let action = r#"{"type":"log","message":"tick"}"#;

        sqlx::query(
            "INSERT INTO tasks (id, schedule, action, enabled, created_at) VALUES (?, ?, ?, 1, ?)",
        )
        .bind(&id)
        .bind("* * * * *")
        .bind(action)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        execute_due_tasks(&pool).await.unwrap();

        // Verify a run was recorded
        let runs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_runs WHERE task_id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(runs, 1);

        // Verify last_run_at was updated
        let last_run: Option<i64> =
            sqlx::query_scalar("SELECT last_run_at FROM tasks WHERE id = ?")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(last_run.is_some());
    }

    #[test]
    fn parse_log_action() {
        let action = TaskAction::parse(r#"{"type":"log","message":"hello"}"#).unwrap();
        assert!(matches!(action, TaskAction::Log { message } if message == "hello"));
    }

    #[test]
    fn parse_http_action() {
        let action =
            TaskAction::parse(r#"{"type":"http","url":"https://example.com","body":null}"#)
                .unwrap();
        assert!(
            matches!(action, TaskAction::Http { url, body: None } if url == "https://example.com")
        );
    }

    #[test]
    fn is_due_never_run_returns_true() {
        let now = unix_timestamp();
        assert!(is_due("* * * * *", None, now));
    }

    #[test]
    fn is_due_recently_run_returns_false() {
        let now = unix_timestamp();
        // If it ran 5 seconds ago and fires every minute, it's not due
        assert!(!is_due("0 0 1 1 *", Some(now - 5), now));
    }
}
