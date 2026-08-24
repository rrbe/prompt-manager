use rusqlite::{Transaction, params};

use crate::error::Result;

pub(super) fn replace(
    transaction: &Transaction<'_>,
    prompt_id: i64,
    tags: &[String],
) -> Result<()> {
    transaction.execute("DELETE FROM prompt_tags WHERE prompt_id = ?1", [prompt_id])?;

    for tag in tags {
        transaction.execute(
            "INSERT INTO tags(name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            [tag],
        )?;
        let tag_id: i64 =
            transaction.query_row("SELECT id FROM tags WHERE name = ?1", [tag], |row| {
                row.get(0)
            })?;
        transaction.execute(
            "INSERT INTO prompt_tags(prompt_id, tag_id) VALUES (?1, ?2)",
            params![prompt_id, tag_id],
        )?;
    }

    cleanup_orphans(transaction)
}

pub(super) fn get(transaction: &Transaction<'_>, prompt_id: i64) -> Result<Vec<String>> {
    let mut statement = transaction.prepare(
        "SELECT tags.name\n\
         FROM tags\n\
         JOIN prompt_tags ON prompt_tags.tag_id = tags.id\n\
         WHERE prompt_tags.prompt_id = ?1\n\
         ORDER BY tags.name",
    )?;
    let rows = statement.query_map([prompt_id], |row| row.get(0))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub(super) fn cleanup_orphans(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute(
        "DELETE FROM tags WHERE NOT EXISTS (\n\
            SELECT 1 FROM prompt_tags WHERE prompt_tags.tag_id = tags.id\n\
        )",
        [],
    )?;
    Ok(())
}
