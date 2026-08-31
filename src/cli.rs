use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    str::FromStr,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

use crate::prompt::template::is_valid_variable_name;

#[derive(Debug, Parser)]
#[command(name = "pm", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a prompt using an external editor.
    Add(AddArgs),
    /// Edit an existing prompt using an external editor.
    Edit(NameArgs),
    /// Remove a prompt.
    Rm(RemoveArgs),
    /// Get a prompt with variables and piped input.
    #[command(
        after_long_help = "Examples:\n  # Start an interactive Codex session with a stored prompt\n  codex \"$(pm get prompt-name)\"\n\n  # Run a stored prompt non-interactively with Codex Exec\n  pm get prompt-name | codex exec -"
    )]
    Get(GetArgs),
    /// Execute a prompt using its configured command.
    #[command(
        after_long_help = "Examples:\n  # Execute the configured command\n  pm exec prompt-name\n\n  # Append arguments to the configured command\n  pm exec prompt-name -- --model gpt-5.4"
    )]
    Exec(ExecArgs),
    /// List prompts with edit and usage times.
    List(ListArgs),
    /// Search prompt names, descriptions, and bodies.
    Search(SearchArgs),
    /// Import a Markdown prompt file.
    Import(ImportArgs),
    /// Export a prompt as Markdown.
    Export(ExportArgs),
    /// Add or remove a prompt from favorites.
    Favorite(FavoriteArgs),
    /// Inspect the saved versions of a prompt.
    History(HistoryArgs),
    /// Check for or install the latest pm release.
    Update(UpdateArgs),
    /// Generate shell completions.
    #[command(
        long_about = "Generate shell completions.\n\nThe generated script is written to stdout and is not installed automatically. Without --dynamic, it completes commands and options. With --dynamic, it also reads Prompt names from SQLite at completion time. Regenerate installed scripts after upgrading or moving pm.",
        after_long_help = "Examples:\n  # Preview static Zsh completions\n  pm completions zsh | less\n\n  # Install dynamic Zsh completions\n  mkdir -p ~/.zfunc\n  pm completions zsh --dynamic > ~/.zfunc/_pm\n  # Add `fpath=(~/.zfunc $fpath)` before `compinit` in ~/.zshrc.\n\n  # Install dynamic Bash completions (requires bash-completion)\n  mkdir -p ~/.local/share/bash-completion/completions\n  pm completions bash --dynamic > ~/.local/share/bash-completion/completions/pm\n\n  # Install dynamic Fish completions\n  mkdir -p ~/.config/fish/completions\n  pm completions fish --dynamic > ~/.config/fish/completions/pm.fish\n\n  # Alternatively, load dynamic Zsh completions on every shell startup\n  source <(pm completions zsh --dynamic)"
    )]
    Completions(CompletionsArgs),
}

#[derive(Debug, Args)]
pub struct NameArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

    /// Remove without confirmation.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    #[command(flatten)]
    pub prompt: PromptArgs,
}

#[derive(Debug, Args)]
pub struct ExecArgs {
    #[command(flatten)]
    pub prompt: PromptArgs,

    /// Append arguments to the configured command.
    #[arg(last = true, value_name = "ARG")]
    pub arguments: Vec<OsString>,
}

#[derive(Debug, Args)]
pub struct PromptArgs {
    /// Prompt name; required unless --id or --pick is used.
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    #[arg(
        required_unless_present_any = ["id", "pick"],
        conflicts_with_all = ["id", "pick"]
    )]
    pub name: Option<String>,

    /// Select a prompt by ID instead of providing NAME.
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..), conflicts_with = "pick")]
    pub id: Option<i64>,

    /// Select a prompt using fzf instead of providing NAME.
    #[arg(long)]
    pub pick: bool,

    /// Set a variable as KEY=VALUE.
    #[arg(short = 'v', long = "var", value_name = "KEY=VALUE")]
    pub variables: Vec<VariableAssignment>,

    /// Prompt for each missing variable using a multiline terminal form.
    #[arg(short, long)]
    pub interactive: bool,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub query: String,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// List only prompts in this group; include the trailing slash.
    #[arg(value_name = "GROUP/")]
    pub group: Option<String>,

    /// List only prompts with this tag.
    #[arg(long)]
    pub tag: Option<String>,

    /// List only favorite prompts.
    #[arg(long)]
    pub favorite: bool,

    /// Sort the result.
    #[arg(long, value_enum, default_value_t = ListSort::Name)]
    pub sort: ListSort,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ListSort {
    #[default]
    Name,
    Updated,
    Used,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Prompt name, or output directory when using --all.
    pub target: PathBuf,

    /// Export every prompt to the target directory.
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct FavoriteArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

    /// Remove the prompt from favorites.
    #[arg(long)]
    pub remove: bool,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

    #[command(subcommand)]
    pub command: Option<HistoryCommand>,
}

#[derive(Debug, Subcommand)]
pub enum HistoryCommand {
    /// Diff two saved versions of this prompt.
    Diff(HistoryDiffArgs),
}

#[derive(Debug, Args)]
pub struct HistoryDiffArgs {
    /// Older version number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    pub old: i64,

    /// Newer version number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    pub new: i64,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Check for a newer release without installing it.
    #[arg(long)]
    pub check: bool,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: CompletionShell,

    /// Generate a completion script that reads Prompt names at completion time.
    #[arg(long)]
    pub dynamic: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

impl CompletionShell {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VariableAssignment {
    pub key: String,
    pub value: String,
}

impl FromStr for VariableAssignment {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (key, value) = split_assignment(value)?;
        Ok(Self {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }
}

fn split_assignment(value: &str) -> std::result::Result<(&str, &str), String> {
    let (key, value) = value
        .split_once('=')
        .ok_or_else(|| "expected KEY=VALUE".to_owned())?;
    if !is_valid_variable_name(key) {
        return Err(format!("invalid variable name: {key}"));
    }
    Ok((key, value))
}

fn prompt_name_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let Some(database) = completion_database() else {
        return Vec::new();
    };
    let Ok(names) = database.list_prompt_names() else {
        return Vec::new();
    };
    names
        .into_iter()
        .filter(|name| name.starts_with(current))
        .map(CompletionCandidate::new)
        .collect()
}

fn completion_database() -> Option<crate::db::Database> {
    let path = crate::paths::database_path().ok()?;
    if !path.is_file() {
        return None;
    }
    crate::db::Database::open_read_only(&path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variable_values_containing_equals() {
        let value: VariableAssignment = "query=a=b".parse().unwrap();
        assert_eq!(value.key, "query");
        assert_eq!(value.value, "a=b");
    }

    #[test]
    fn rejects_invalid_assignment() {
        assert!("not-an-assignment".parse::<VariableAssignment>().is_err());
        assert!("1key=value".parse::<VariableAssignment>().is_err());
    }

    #[test]
    fn parses_update_check() {
        let cli = Cli::try_parse_from(["pm", "update", "--check"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Update(UpdateArgs { check: true })
        ));
    }
}
