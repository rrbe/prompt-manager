use crate::{
    cli::{ListArgs, ListSort},
    db::{Database, PromptListEntry},
    error::{Error, Result},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::write_stdout;

pub fn run(arguments: ListArgs, database: &Database) -> Result<()> {
    let tag = match arguments.tag {
        Some(tag) => {
            let tag = tag.trim();
            if tag.is_empty() {
                return Err(Error::Message("tag must not be empty".into()));
            }
            Some(tag.to_owned())
        }
        None => None,
    };
    let mut prompts = database.list_prompts_filtered(tag.as_deref(), arguments.favorite)?;
    sort(&mut prompts, arguments.sort);
    if prompts.is_empty() {
        return Ok(());
    }

    let lines = prompts
        .into_iter()
        .map(|prompt| {
            if arguments.long {
                Ok(format!(
                    "{}\t{}\t{}",
                    prompt.name,
                    format_timestamp(prompt.updated_at)?,
                    prompt
                        .last_used_at
                        .map(format_timestamp)
                        .transpose()?
                        .unwrap_or_else(|| "-".into())
                ))
            } else {
                Ok(prompt.name)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    write_stdout(&format!("{}\n", lines.join("\n")))
}

fn sort(prompts: &mut [PromptListEntry], sort: ListSort) {
    prompts.sort_by(|left, right| match sort {
        ListSort::Name => left.name.cmp(&right.name),
        ListSort::Updated => right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.name.cmp(&right.name)),
        ListSort::Used => right
            .last_used_at
            .cmp(&left.last_used_at)
            .then_with(|| left.name.cmp(&right.name)),
    });
}

pub(super) fn format_timestamp(timestamp: i64) -> Result<String> {
    OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| Error::Message(format!("timestamp is out of range: {timestamp}")))?
        .format(&Rfc3339)
        .map_err(|error| Error::Message(format!("failed to format timestamp: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_unused_prompts_after_used_prompts() {
        let mut prompts = vec![
            PromptListEntry {
                name: "unused".into(),
                updated_at: 1,
                last_used_at: None,
            },
            PromptListEntry {
                name: "used".into(),
                updated_at: 1,
                last_used_at: Some(2),
            },
        ];
        sort(&mut prompts, ListSort::Used);
        assert_eq!(prompts[0].name, "used");
    }

    #[test]
    fn formats_timestamps_as_utc_rfc3339() {
        assert_eq!(format_timestamp(0).unwrap(), "1970-01-01T00:00:00Z");
    }
}
