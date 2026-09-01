# PM

`pm` is a local, pipe-friendly command-line prompt manager that uses SQLite to store and search reusable prompts.

TLDR:

```bash
pm add work/code-review                   # save prompt，/ is the optional grouping separator
pm list                                   # list saved prompts
pm get code-review                        # print the prompt `code-review` to stdio
pm get code-review | codex exec -
pm exec code-review                       # run the prompt's configured command
codex "$(pm get prompt-name)"
pm edit code-review
pm rm code-review
```

## Installation

The stable Rust toolchain is required. SQLite is bundled, so no system SQLite library is needed.

```bash
make install                                  # Install to /usr/local/bin by default
make install INSTALL_DIR="$HOME/.local/bin" # Install to a user directory
cargo install --path .                        # Install to ~/.cargo/bin
```

`make install` does not invoke `sudo` automatically.

## Quick Start

```bash
pm add code-review
pm add work/week-report
generate-prompt | pm add generated
pm add from-file < prompt-body.md
generate-prompt | pm add generated --no-edit
pm list
pm get code-review
pm get code-review | codex exec -
pm exec code-review
codex "$(pm get prompt-name)"
pm edit code-review
pm rm code-review
```

`add` uses piped or redirected stdin as the initial prompt body before opening the editor. Pass `--no-edit` to validate and save directly instead. `add` and `edit` select an editor in this order: `$VISUAL`, `$EDITOR`, then `vi`. `remove` is an alias for `rm`; both ask for terminal confirmation by default, and `--force` skips it.

Prompts use Markdown with YAML front matter:

```markdown
---
name: code-review
description: Review source code
tags:
  - coding
  - review
exec: codex exec -
---

Review the following code:

{{input}}
```

`name` must be unique; `description`, `tags`, and `exec` are optional. Changing `name` also renames the prompt. Use `/` between name segments to organize prompts into groups, such as `work/week-report`.

## Retrieval and Templates

Select a prompt by name, by the ID shown by `list`, or with the external `fzf` command:

```bash
pm get code-review
pm get --id 1
pm get --pick
```

Use `-v` to assign template variables:

```bash
pm get code-review \
  -v language=rust \
  -v focus=correctness
```

Use `--interactive` to fill each missing variable in template order. Values may span multiple lines; finish each value with a line containing only `EOF`. Surrounding whitespace is trimmed from each value. Prompts are written to stderr, and only the rendered result is written to stdout:

```bash
pm get weekly-report --interactive
pm get weekly-report --interactive -v week=2026-W35 | codex exec -
```

Send a stored prompt to Codex as an interactive initial prompt or through non-interactive `codex exec`:

```bash
codex "$(pm get prompt-name)"
pm get prompt-name | codex exec -
```

`{{input}}` reads from stdin, so `pm` composes directly with other commands:

```bash
some-command | pm get code-review | codex exec -
pm get code-review | claude -p
```

Prompts can also reference other prompts:

```text
{{prompt:senior-engineer}}

{{prompt:security-guidelines}}

{{input}}
```

`get` expands references before substituting variables. The command fails when a reference is missing, references form a cycle, or a variable has no value. Variable names must match `[a-zA-Z_][a-zA-Z0-9_-]*`; whitespace immediately inside `{{` and `}}` is allowed. Invalid template expressions are rejected when prompts are added, edited, or imported.

## Execution

Set `exec` in a prompt's front matter to run the rendered prompt without rebuilding the command line each time:

```markdown
---
name: code-review
exec: codex exec -
---

Review the following code:

{{input}}
```

`pm exec` supports the same selectors and template options as `pm get`. The rendered prompt is written to the configured command's stdin:

```bash
pm exec code-review
pm exec --id 1
pm exec --pick
some-command | pm exec code-review -v language=rust
```

Arguments after `--` are appended to the configured command:

```bash
pm exec code-review -- --model gpt-5.4
```

The command is split into an executable and arguments using shell-style quoting and is launched directly. It inherits `pm`'s working directory, environment, stdout, and stderr. `pm exec` returns the command's exit code.

## Listing and Search

```bash
pm list
pm list work/
pm list --tag coding
pm list --favorite
pm list --sort updated
pm list --sort updated --reverse
pm list --sort used
pm search mongo
```

`list` displays stable IDs accepted by `pm get --id ID`, prompt names, local update times to the minute, and relative last-use times in an aligned table. Pass a group ending in `/`, such as `pm list work/`, to recursively list prompts in that group while keeping their full names. Nested groups are supported. `--sort updated` sorts by update time, and `--sort used` sorts by most recent use. Pass `-r` or `--reverse` to reverse the selected sort order.

When stdout is a terminal, `list` sends its table through `$PAGER`, or `less -FRX` when `$PAGER` is unset. Piped and redirected output remains complete plain text. Set `PAGER=cat` to disable interactive paging.

Use favorites together with `list --favorite`:

```bash
pm favorite code-review
pm favorite code-review --remove
```

## History

Creating a prompt and editing its content or metadata creates a version. An edit with no changes does not create a new version.

```bash
pm history code-review
pm history code-review diff 1 3
```

History lists versions in an aligned table. Its timestamps use local time to the minute, matching `pm list`.

## Import and Export

```bash
pm export code-review > code-review.md
pm import code-review.md
pm export --all ./prompts/
```

Importing a prompt whose name already exists does not overwrite the existing prompt.

## Shell Completion

Generate static completion scripts for Bash, Zsh, or Fish:

```bash
pm completions zsh > _pm
```

Dynamic completion also reads prompt names from SQLite:

```bash
pm completions zsh --dynamic > _pm
```

## Updating

Check GitHub Releases for a newer version without changing the installed binary:

```bash
pm update --check
```

Download the release archive for the current platform, verify its published SHA-256 checksum, and replace the current binary:

```bash
pm update
```

Update status is written to stderr, leaving stdout available for pipelines. The command does not modify the prompt database.
If GitHub's anonymous API rate limit is exhausted, set `GH_TOKEN` or `GITHUB_TOKEN` before retrying.

## Data Storage

The database is stored at `$HOME/.local/share/pm/pm.db` by default. When `XDG_DATA_HOME` is set, it uses `$XDG_DATA_HOME/pm/pm.db` instead.

## Development

```bash
make fmt
make check
make build-release
```

Run `pm --help` or `pm <command> --help` to see all commands and options.
