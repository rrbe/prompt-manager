# PM

`pm` 是一个本地、Pipe 友好的命令行 Prompt Manager，使用 SQLite 保存和搜索可复用的 Prompt。

## 安装

需要 Rust stable toolchain。SQLite 已静态编译，不需要安装系统 SQLite library。

```bash
make install                                  # 默认安装到 /usr/local/bin
make install INSTALL_DIR="$HOME/.local/bin" # 安装到用户目录
cargo install --path .                        # 安装到 ~/.cargo/bin
```

`make install` 不会自动调用 `sudo`。

## 快速开始

```bash
pm add code-review
pm list
pm get code-review
pm get code-review | codex exec -
pm edit code-review
pm rm code-review
```

`add` 和 `edit` 按 `$VISUAL`、`$EDITOR`、`vi` 的优先级选择编辑器。`rm` 默认要求终端确认，使用 `--force` 可跳过确认。

编辑器中的 Prompt 使用 Markdown 和 YAML front matter：

```markdown
---
name: code-review
description: Review source code
tags:
  - coding
  - review
---

Review the following code:

{{input}}
```

`name` 必须唯一；`description` 和 `tags` 可选。修改 `name` 会同时重命名 Prompt。

## 获取与模板

可以通过名称、`list` 显示的 ID，或者外部 `fzf` 选择 Prompt：

```bash
pm get code-review
pm get --id 1
pm get --pick
```

使用 `-v` 为模板变量赋值：

```bash
pm get code-review \
  -v language=rust \
  -v focus=correctness
```

Prompt 中的 `{{input}}` 从 stdin 获取内容，因此可以直接组合其他命令：

```bash
some-command | pm get code-review | codex exec -
pm get code-review | claude -p
```

Prompt 还可以引用其他 Prompt：

```text
{{prompt:senior-engineer}}

{{prompt:security-guidelines}}

{{input}}
```

`get` 会先展开引用，再替换变量。引用不存在、形成循环或缺少变量时命令失败。

## 列表与搜索

```bash
pm list
pm list --tag coding
pm list --favorite
pm list --long --sort updated
pm list --long --sort used
pm search mongo
```

`list` 会显示可供 `pm get --id ID` 使用的稳定 ID。`--sort updated` 按更新时间排序，`--sort used` 按最近使用时间排序，`--long` 显示对应时间。

收藏可以配合 `list --favorite` 使用：

```bash
pm favorite code-review
pm favorite code-review --remove
```

## 历史

创建和每次有实际内容或 metadata 变化的编辑都会生成版本；无变化的编辑不会生成新版本。

```bash
pm history code-review
pm history code-review diff 1 3
```

## Import 与 Export

```bash
pm export code-review > code-review.md
pm import code-review.md
pm export --all ./prompts/
```

Import 名称冲突时不会覆盖已有 Prompt。

## Shell Completion

生成 bash、zsh 或 fish 的静态补全脚本：

```bash
pm completions zsh > _pm
```

动态补全还会从 SQLite 读取 Prompt 名称：

```bash
pm completions zsh --dynamic > _pm
```

## 数据存储

数据库默认位于 `$HOME/.local/share/pm/pm.db`；设置 `XDG_DATA_HOME` 后使用 `$XDG_DATA_HOME/pm/pm.db`。

## 开发

```bash
make fmt
make check
make build-release
```

查看完整命令和参数请使用 `pm --help` 或 `pm <command> --help`。
