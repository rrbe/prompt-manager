# PM

English | [简体中文](README_CN.md)

`pm` is a local, pipe-friendly command-line prompt manager that uses SQLite to store and search reusable prompts.

## TLDR

```bash
# Save a prompt; / is the optional group separator
pm add work/code-review

# Create a prompt from piped content
generate-prompt | pm add generated
pm add from-file < prompt-body.md

# List saved prompts
pm list

# Print a saved prompt, and pipe it to an agent
pm get code-review
pm get code-review | codex exec -
codex "$(pm get prompt-name)"
# If an execution command is configured (like systemd's ExecStart), run it directly
pm exec code-review

# Edit / remove a prompt
pm edit code-review
pm rm code-review
```

## Prompt Structure

A prompt is Markdown with YAML front matter:

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

- `name` must be unique, but can be changed at any time
- `description`, `tags`, and `exec` are optional settings

## Usage

### 1. Basic Usage

```bash
pm --help
pm <COMMAND> --help
```

### 2. Variables

Prompts support variables: names must match `[a-zA-Z_][a-zA-Z0-9_-]*`, variables are wrapped in double braces, and default values are supported

```markdown
---
name: system-check
---

SSH to the {{hostname=us-east-1}} host and check the following:

1. System load over the past {{time=15min}}
2. Disk usage
```

Assign values to variables with `-v`:

```bash
pm get system-check \
  -v hostname=us-west-2 \
  -v time=1h
```

Use `-i` to fill in missing variables interactively, one at a time. Values may span multiple lines; after entering each value, enter a line with `EOF` to finish it, or press <Ctrl>-D. Prompts are written to stderr, and the rendered result is written to stdout:

```bash
pm get system-check -i
pm get system-check -i -v time=1h | codex exec -
```

Prompts can also reference other prompts:

```text
{{prompt:senior-engineer}}

{{prompt:security-guidelines}}

{{input}}
```

A literal default value can be defined after the first `=`:

```text
Language: {{ language=rust }}
Endpoint: {{ endpoint=https://example.com?a=1&b=2 }}
```

### 3. Exec Command

Set `exec` in a prompt's front matter to define its default execution command:

```markdown
---
name: code-review
exec: codex exec -
---

Review the following code:

{{input}}
```

```bash
pm exec code-review
```

Arguments after `--` are appended to the configured command:

```bash
pm exec code-review -- --model gpt-5.4
```

### 4. Listing and Search

```bash
pm list
# List only prompts under this prefix
pm list work/
pm list --tag coding
pm list --full
pm list --quiet
pm list --sort updated
pm list --sort updated -r
pm list --sort used
pm search mongo
```

### 5. History

Creating a prompt and editing its content or metadata each create a version

```bash
pm history code-review
pm history code-review diff 1 3
```

### 6. Import and Export

```bash
pm export code-review > code-review.md
pm import code-review.md
pm export --all ./prompts/
```

### 7. Shell Completion

Generate static completion scripts for Bash, Zsh, or Fish:

```bash
pm completions zsh > _pm
```

Dynamic completion reads prompt names from SQLite:

```bash
pm completions zsh --dynamic > _pm

pm get f # if a prompt named foo exists, Tab completes it
```

### 8. Updating

```bash
pm update
pm update --check
```

### 9. Data Storage

The database is stored at `$HOME/.local/share/pm/pm.db` by default. When `XDG_DATA_HOME` is set, `$XDG_DATA_HOME/pm/pm.db` is used instead.

## Installation

```bash
make install                                  # Install to /usr/local/bin by default
make install INSTALL_DIR="$HOME/.local/bin"   # Install to a user directory
cargo install --path .                        # Install to ~/.cargo/bin
```

## Development

```bash
make fmt
make check
make build-release
```
