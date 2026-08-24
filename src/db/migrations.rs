use rusqlite::params;

use crate::error::Result;

use super::Database;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: include_str!("../../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "favorites_and_history",
        sql: include_str!("../../migrations/0002_favorites_and_history.sql"),
    },
    Migration {
        version: 3,
        name: "monotonic_prompt_ids",
        sql: include_str!("../../migrations/0003_monotonic_prompt_ids.sql"),
    },
];

impl Database {
    pub(crate) fn migrate(&mut self) -> Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (\n\
                version INTEGER PRIMARY KEY,\n\
                name TEXT NOT NULL,\n\
                applied_at INTEGER NOT NULL\n\
            );",
        )?;

        for migration in MIGRATIONS {
            let applied = self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [migration.version],
                |row| row.get::<_, bool>(0),
            )?;
            if applied {
                continue;
            }

            let transaction = self.connection.transaction()?;
            transaction.execute_batch(migration.sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![migration.version, migration.name, super::now_timestamp()],
            )?;
            transaction.commit()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::PromptInput;
    use rusqlite::Connection;

    #[test]
    fn migrations_are_idempotent() {
        let mut database = Database::in_memory().unwrap();
        database.migrate().unwrap();
        let count: i64 = database
            .connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn later_migrations_backfill_history_and_seed_prompt_ids() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (\n\
                    version INTEGER PRIMARY KEY,\n\
                    name TEXT NOT NULL,\n\
                    applied_at INTEGER NOT NULL\n\
                );",
            )
            .unwrap();
        connection.execute_batch(MIGRATIONS[0].sql).unwrap();
        connection
            .execute("INSERT INTO schema_migrations VALUES (1, 'initial', 1)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO prompts(name, content, created_at, updated_at)\n\
                 VALUES ('existing', 'body', 1, 2)",
                [],
            )
            .unwrap();

        let mut database = Database { connection };
        database.migrate().unwrap();
        let history = database.prompt_history("existing").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].created_at, 2);
        assert!(!database.get_prompt("existing").unwrap().favorite);

        database.delete_prompt("existing").unwrap();
        database
            .create_prompt(&PromptInput {
                name: "new".into(),
                description: None,
                content: "body".into(),
                tags: Vec::new(),
            })
            .unwrap();
        assert_eq!(database.get_prompt("new").unwrap().id, 2);
    }
}
