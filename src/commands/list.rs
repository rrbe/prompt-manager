use crate::{
    cli::{ListArgs, ListSort},
    db::{Database, PromptListEntry},
    error::{Error, Result},
    prompt::validate_group,
};
use std::time::Duration;

use time::{OffsetDateTime, UtcOffset};
use timeago::{Formatter, TimeUnit};

use super::{current_local_offset, format_local_timestamp, format_table, write_paged_stdout};

pub fn run(arguments: ListArgs, database: &Database) -> Result<()> {
    if let Some(group) = arguments.group.as_deref() {
        validate_group(group)?;
    }
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
    let mut prompts = database.list_prompts_filtered(
        arguments.group.as_deref(),
        tag.as_deref(),
        arguments.favorite,
    )?;
    sort(&mut prompts, arguments.sort);
    if prompts.is_empty() {
        return Ok(());
    }

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let local_offset = current_local_offset()?;
    write_paged_stdout(&render_table(&prompts, now, local_offset)?)
}

fn render_table(prompts: &[PromptListEntry], now: i64, local_offset: UtcOffset) -> Result<String> {
    const HEADERS: [&str; 4] = ["ID", "NAME", "UPDATED AT", "LAST USE"];

    let mut relative_time = Formatter::new();
    relative_time.min_unit(TimeUnit::Minutes);
    let rows = prompts
        .iter()
        .map(|prompt| {
            Ok([
                prompt.id.to_string(),
                prompt.name.clone(),
                format_local_timestamp(prompt.updated_at, local_offset)?,
                prompt
                    .last_used_at
                    .map(|timestamp| format_relative_timestamp(timestamp, now, &relative_time))
                    .unwrap_or_else(|| "-".into()),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(format_table(&HEADERS, &rows))
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

fn format_relative_timestamp(timestamp: i64, now: i64, formatter: &Formatter) -> String {
    let elapsed = now.saturating_sub(timestamp).max(0) as u64;
    formatter.convert(Duration::from_secs(elapsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_unused_prompts_after_used_prompts() {
        let mut prompts = vec![
            PromptListEntry {
                id: 1,
                name: "unused".into(),
                updated_at: 1,
                last_used_at: None,
            },
            PromptListEntry {
                id: 2,
                name: "used".into(),
                updated_at: 1,
                last_used_at: Some(2),
            },
        ];
        sort(&mut prompts, ListSort::Used);
        assert_eq!(prompts[0].name, "used");
    }

    #[test]
    fn formats_relative_timestamps() {
        let mut formatter = Formatter::new();
        formatter.min_unit(TimeUnit::Minutes);

        assert_eq!(format_relative_timestamp(99, 100, &formatter), "now");
        assert_eq!(format_relative_timestamp(101, 100, &formatter), "now");
        assert_eq!(
            format_relative_timestamp(40, 100, &formatter),
            "1 minute ago"
        );
        assert_eq!(
            format_relative_timestamp(0, 7_200, &formatter),
            "2 hours ago"
        );
        assert_eq!(
            format_relative_timestamp(0, 172_800, &formatter),
            "2 days ago"
        );
    }

    #[test]
    fn formats_updated_timestamps_in_local_time_to_the_minute() {
        let offset = UtcOffset::from_hms(8, 0, 0).unwrap();
        assert_eq!(
            format_local_timestamp(0, offset).unwrap(),
            "1970-01-01 08:00"
        );
    }

    #[test]
    fn renders_an_aligned_table_with_timestamp_columns() {
        let prompts = vec![
            PromptListEntry {
                id: 2,
                name: "alpha".into(),
                updated_at: 1,
                last_used_at: Some(2),
            },
            PromptListEntry {
                id: 10,
                name: "longer-name".into(),
                updated_at: 3,
                last_used_at: None,
            },
        ];

        assert_eq!(
            render_table(&prompts, 7_201, UtcOffset::UTC).unwrap(),
            "ID  NAME         UPDATED AT        LAST USE\n\
             ──  ───────────  ────────────────  ──────────\n\
             2   alpha        1970-01-01 00:00  1 hour ago\n\
             10  longer-name  1970-01-01 00:00  -\n"
        );
    }
}
