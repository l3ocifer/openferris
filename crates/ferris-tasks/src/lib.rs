use ferris_common::{unix_timestamp, FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub schedule: String,
    pub action: String,
    pub enabled: bool,
    pub created_at: i64,
}

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
    pub async fn schedule_task(&self, schedule: &str, action: &str) -> Result<Task> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE enabled = 1")
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
        })
    }

    /// List all tasks.
    pub async fn list_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT id, schedule, action, enabled, created_at
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
}

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Task {
    let enabled: i32 = row.get("enabled");
    Task {
        task_id: row.get("id"),
        schedule: row.get("schedule"),
        action: row.get("action"),
        enabled: enabled != 0,
        created_at: row.get("created_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                schedule TEXT NOT NULL,
                action TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
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

        let task = scheduler
            .schedule_task("0 * * * *", "send heartbeat")
            .await
            .unwrap();
        assert_eq!(task.schedule, "0 * * * *");
        assert_eq!(task.action, "send heartbeat");
        assert!(task.enabled);

        let tasks = scheduler.list_tasks().await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, task.task_id);
    }

    #[tokio::test]
    async fn cancel_removes_task() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 10);

        let task = scheduler
            .schedule_task("*/5 * * * *", "cleanup")
            .await
            .unwrap();
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

        scheduler.schedule_task("0 * * * *", "a").await.unwrap();
        scheduler.schedule_task("0 * * * *", "b").await.unwrap();
        let err = scheduler
            .schedule_task("0 * * * *", "c")
            .await
            .unwrap_err();
        assert!(matches!(err, FerrisError::CapacityExceeded(_)));
    }

    #[tokio::test]
    async fn multiple_tasks_all_returned() {
        let pool = test_pool().await;
        let scheduler = TaskScheduler::new(pool, 10);

        let t1 = scheduler.schedule_task("0 * * * *", "first").await.unwrap();
        let t2 = scheduler.schedule_task("0 * * * *", "second").await.unwrap();

        let tasks = scheduler.list_tasks().await.unwrap();
        assert_eq!(tasks.len(), 2);
        let ids: Vec<&str> = tasks.iter().map(|t| t.task_id.as_str()).collect();
        assert!(ids.contains(&t1.task_id.as_str()));
        assert!(ids.contains(&t2.task_id.as_str()));
    }
}
