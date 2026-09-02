# PM

[English](README.md) | 简体中文

`pm` 是一个本地、管道友好的命令行提示词（prompt）管理工具，使用 SQLite 存储和检索可复用的提示词。

## TLDR

```bash
# 保存提示词，/ 是可选的分组分隔符
pm add work/code-review

# 从管道读取提示词内容创建
generate-prompt | pm add generated
pm add from-file < prompt-body.md

# 列出已保存的提示词
pm list

# 查看已保存的提示词，通过管道交给 agent 执行
pm get code-review
pm get code-review | codex exec -
codex "$(pm get prompt-name)"
# 如果设置了“执行方式”（类似 systemd 的 ExecStart），可以直接执行
pm exec code-review

# 编辑/删除提示词
pm edit code-review
pm rm code-review
```

## Prompt 结构

一条 Prompt 是带 YAML front matter 的 Markdown：

```markdown
---
name: code-review
description: 审查源代码
tags:
  - coding
  - review
exec: codex exec -
---

审查以下代码：

{{input}}
```

- `name` 必须唯一，但可以随时修改
- `description`、`tags` 和 `exec` 等为可选设置

## 使用

### 1. 基本用法

```bash
pm --help
pm <COMMAND> --help
```

### 2. 变量

prompt 支持变量，变量名须匹配 `[a-zA-Z_][a-zA-Z0-9_-]*`，用双大括号包裹，支持默认值

```markdown
---
name: system-check
---

SSH 到 {{hostname=us-east-1}} 机器上检查下列指标：

1. 过去 {{time=15min}} 系统负载
2. 磁盘使用率
```

使用时通过 `-v` 为变量赋值：

```bash
pm get system-check \
  -v hostname=us-west-2 \
  -v time=1h
```

也可以使用 `-i` 交互式逐一填写缺失的变量。值可以跨多行；每输入完一个值后，输入一行 `EOF` 结束该值，或按 <Ctrl>-D。提示会输出到 stderr，渲染后的结果输出到 stdout：

```bash
pm get system-check -i
pm get system-check -i -v time=1h | codex exec -
```

提示词还可以引用其他提示词：

```text
{{prompt:senior-engineer}}

{{prompt:security-guidelines}}

{{input}}
```

可以在第一个 `=` 之后定义字面量默认值：

```text
Language: {{ language=rust }}
Endpoint: {{ endpoint=https://example.com?a=1&b=2 }}
```

### 3. Exec 命令

在提示词的 front matter 中设置 `exec`，即可指定默认执行方式：

```markdown
---
name: code-review
exec: codex exec -
---

审查以下代码：

{{input}}
```

```bash
pm exec code-review
```

可以用 `--` 追加覆盖配置的命令：

```bash
pm exec code-review -- --model gpt-5.4
```

### 4. 列表与搜索

```bash
pm list
# 只列出该前缀下的提示词
pm list work/
pm list --tag coding
pm list --full
pm list --quiet
pm list --sort updated
pm list --sort updated -r
pm list --sort used
pm search mongo
```

### 5. 历史记录

创建提示词以及编辑其内容或元数据都会创建一个版本

```bash
pm history code-review
pm history code-review diff 1 3
```

### 6. 导入与导出

```bash
pm export code-review > code-review.md
pm import code-review.md
pm export --all ./prompts/
```

### 7. Shell 补全

为 Bash、Zsh 或 Fish 生成静态补全脚本：

```bash
pm completions zsh > _pm
```

动态补全会从 SQLite 读取提示词名称：

```bash
pm completions zsh --dynamic > _pm

pm get f # 如果有名为 foo 的提示词，按 tab 会补全
```

### 8. 更新

```bash
pm update
pm update --check
```

### 9. 数据存储

数据库默认存储在 `$HOME/.local/share/pm/pm.db`。当设置了 `XDG_DATA_HOME` 时，则使用 `$XDG_DATA_HOME/pm/pm.db`。

## 安装

```bash
make install                                  # 默认安装到 /usr/local/bin
make install INSTALL_DIR="$HOME/.local/bin"   # 安装到用户目录
cargo install --path .                        # 安装到 ~/.cargo/bin
```

## 开发

```bash
make fmt
make check
make build-release
```
