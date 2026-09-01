use crate::{
    cli::{ListArgs, ListSort},
    db::{Database, PromptListEntry},
    error::{Error, Result},
    prompt::validate_group,
};
use std::time::Duration;

use anstyle::{AnsiColor, Style};
use time::{OffsetDateTime, UtcOffset};
use timeago::{Formatter, TimeUnit};

use super::{
    clean_inline, current_local_offset, format_local_timestamp, format_table,
    stdout_supports_color, write_paged_stdout, write_stdout,
};

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
    sort(&mut prompts, arguments.sort, arguments.reverse);
    if prompts.is_empty() {
        return Ok(());
    }

    if arguments.quiet {
        return write_stdout(&render_quiet(&prompts));
    }

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let local_offset = current_local_offset()?;
    let colors_enabled = stdout_supports_color();
    let output = if arguments.full {
        render_full_table(&prompts, now, local_offset, colors_enabled)?
    } else {
        render_default_table(&prompts, now, colors_enabled)
    };
    write_paged_stdout(&output)
}

fn render_default_table(prompts: &[PromptListEntry], now: i64, colors_enabled: bool) -> String {
    const HEADERS: [&str; 4] = ["ID", "NAME", "USES", "LAST USE"];

    let mut relative_time = Formatter::new();
    relative_time.min_unit(TimeUnit::Minutes);
    let rows = prompts
        .iter()
        .map(|prompt| {
            [
                prompt.id.to_string(),
                prompt.name.clone(),
                prompt.use_count.to_string(),
                prompt
                    .last_used_at
                    .map(|timestamp| format_relative_timestamp(timestamp, now, &relative_time))
                    .unwrap_or_else(|| "-".into()),
            ]
        })
        .collect::<Vec<_>>();
    format_table(
        &HEADERS,
        &rows,
        &[
            Style::new().dimmed(),
            AnsiColor::Cyan.on_default(),
            Style::new(),
            Style::new().dimmed(),
        ],
        colors_enabled,
    )
}

fn render_full_table(
    prompts: &[PromptListEntry],
    now: i64,
    local_offset: UtcOffset,
    colors_enabled: bool,
) -> Result<String> {
    const HEADERS: [&str; 10] = [
        "ID",
        "NAME",
        "TAGS",
        "FAVORITE",
        "USES",
        "CREATED AT",
        "UPDATED AT",
        "LAST USE",
        "EXEC",
        "DESCRIPTION",
    ];

    let mut relative_time = Formatter::new();
    relative_time.min_unit(TimeUnit::Minutes);
    let rows = prompts
        .iter()
        .map(|prompt| {
            Ok([
                prompt.id.to_string(),
                prompt.name.clone(),
                format_tags(&prompt.tags),
                if prompt.favorite { "yes" } else { "-" }.into(),
                prompt.use_count.to_string(),
                format_compact_timestamp(prompt.created_at, now, local_offset)?,
                format_compact_timestamp(prompt.updated_at, now, local_offset)?,
                prompt
                    .last_used_at
                    .map(|timestamp| format_relative_timestamp(timestamp, now, &relative_time))
                    .unwrap_or_else(|| "-".into()),
                format_optional_inline(prompt.exec.as_deref()),
                format_optional_inline(prompt.description.as_deref()),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(format_table(
        &HEADERS,
        &rows,
        &[
            Style::new().dimmed(),
            AnsiColor::Cyan.on_default(),
            Style::new(),
            Style::new(),
            Style::new(),
            Style::new().dimmed(),
            Style::new().dimmed(),
            Style::new().dimmed(),
            Style::new(),
            Style::new(),
        ],
        colors_enabled,
    ))
}

fn render_quiet(prompts: &[PromptListEntry]) -> String {
    format!(
        "{}\n",
        prompts
            .iter()
            .map(|prompt| prompt.name.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        "-".into()
    } else {
        tags.iter()
            .map(|tag| clean_inline(tag))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_optional_inline(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(clean_inline)
        .unwrap_or_else(|| "-".into())
}

fn sort(prompts: &mut [PromptListEntry], sort: ListSort, reverse: bool) {
    prompts.sort_by(|left, right| {
        let ordering = match sort {
            ListSort::Name => left.name.cmp(&right.name),
            ListSort::Updated => right.updated_at.cmp(&left.updated_at),
            ListSort::Used => right.last_used_at.cmp(&left.last_used_at),
        };
        let ordering = if reverse {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| left.name.cmp(&right.name))
    });
}

fn format_relative_timestamp(timestamp: i64, now: i64, formatter: &Formatter) -> String {
    let elapsed = now.saturating_sub(timestamp).max(0) as u64;
    formatter.convert(Duration::from_secs(elapsed))
}

fn format_compact_timestamp(timestamp: i64, now: i64, local_offset: UtcOffset) -> Result<String> {
    let local_time = OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| Error::Message(format!("timestamp is out of range: {timestamp}")))?
        .to_offset(local_offset);
    let current_year = OffsetDateTime::from_unix_timestamp(now)
        .map_err(|_| Error::Message(format!("timestamp is out of range: {now}")))?
        .to_offset(local_offset)
        .year();

    if local_time.year() == current_year {
        Ok(format!(
            "{:02}-{:02} {:02}:{:02}",
            local_time.month() as u8,
            local_time.day(),
            local_time.hour(),
            local_time.minute()
        ))
    } else {
        format_local_timestamp(timestamp, local_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(id: i64, name: &str, updated_at: i64, last_used_at: Option<i64>) -> PromptListEntry {
        PromptListEntry {
            id,
            name: name.into(),
            description: None,
            tags: Vec::new(),
            exec: None,
            created_at: updated_at,
            updated_at,
            last_used_at,
            use_count: 0,
            favorite: false,
        }
    }

    #[test]
    fn sorts_unused_prompts_after_used_prompts() {
        let mut prompts = vec![prompt(1, "unused", 1, None), prompt(2, "used", 1, Some(2))];
        sort(&mut prompts, ListSort::Used, false);
        assert_eq!(prompts[0].name, "used");
    }

    #[test]
    fn reverses_the_primary_sort_order_and_keeps_name_ties_ascending() {
        let prompts = vec![
            prompt(1, "alpha", 1, None),
            prompt(2, "beta", 2, Some(2)),
            prompt(3, "gamma", 2, Some(2)),
        ];

        let mut by_name = prompts.clone();
        sort(&mut by_name, ListSort::Name, true);
        assert_eq!(
            by_name
                .iter()
                .map(|prompt| prompt.name.as_str())
                .collect::<Vec<_>>(),
            ["gamma", "beta", "alpha"]
        );

        let mut by_updated = prompts.clone();
        sort(&mut by_updated, ListSort::Updated, true);
        assert_eq!(
            by_updated
                .iter()
                .map(|prompt| prompt.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta", "gamma"]
        );

        let mut by_used = prompts;
        sort(&mut by_used, ListSort::Used, true);
        assert_eq!(
            by_used
                .iter()
                .map(|prompt| prompt.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta", "gamma"]
        );
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
    fn omits_the_year_only_within_the_current_local_year() {
        let offset = UtcOffset::from_hms(8, 0, 0).unwrap();
        assert_eq!(
            format_compact_timestamp(31_507_200, 31_507_200, offset).unwrap(),
            "01-01 00:00"
        );
        assert_eq!(
            format_compact_timestamp(31_507_140, 31_507_200, offset).unwrap(),
            "1970-12-31 23:59"
        );
    }

    #[test]
    fn renders_the_default_table_with_usage_columns() {
        let mut alpha = prompt(2, "alpha", 1, Some(2));
        alpha.use_count = 4;
        let prompts = vec![alpha, prompt(10, "longer-name", 3, None)];

        assert_eq!(
            render_default_table(&prompts, 7_201, false),
            "ID  NAME         USES  LAST USE\n\
             ──  ───────────  ────  ──────────\n\
             2   alpha        4     1 hour ago\n\
             10  longer-name  0     -\n"
        );
    }

    #[test]
    fn renders_full_metadata_as_a_single_line_per_prompt() {
        let mut prompt = prompt(7, "code-review", 31_507_200, Some(31_507_140));
        prompt.description = Some("Review\tsource\ncode".into());
        prompt.tags = vec!["coding".into(), "review".into()];
        prompt.exec = Some("codex exec -".into());
        prompt.use_count = 4;
        prompt.favorite = true;

        let output = render_full_table(
            &[prompt],
            31_507_200,
            UtcOffset::from_hms(8, 0, 0).unwrap(),
            false,
        )
        .unwrap();

        assert!(output.starts_with("ID  NAME"));
        assert!(output.contains("coding, review"));
        assert!(output.contains("yes"));
        assert!(output.contains("codex exec -"));
        assert!(output.contains("Review source code"));
        assert!(!output.contains(['\t', '\r']));
    }

    #[test]
    fn renders_quiet_names_one_per_line() {
        let prompts = vec![
            prompt(1, "alpha", 1, None),
            prompt(2, "work/report", 1, None),
        ];

        assert_eq!(render_quiet(&prompts), "alpha\nwork/report\n");
    }
}
