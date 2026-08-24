# PM

`pm` 是一个命令行 Prompt Manager。使用 SQLite 保存可复用的 Prompt，支持搜索和 Pipe 给其他命令行程序。

示例调用方式

```bash
pm add foobar # 创建名为 foobar 的 Prompt，并在外部编辑器中编写内容
pm list # 列出已保存的 Prompts
pm get foobar # 获取 Prompt 正文并输出到 stdout
pm get foobar | codex exec - # 将保存的 Prompt 交给 Codex
pm get foobar | claude -p # 将保存的 Prompt 交给 Claude
```

## 安装

需要 Rust stable toolchain。SQLite 已静态编译进 binary，不需要安装系统 SQLite library。

### 从源码构建并安装到 `/usr/local/bin/pm`：

```bash
make install
```

### 安装到用户拥有的目录：

```bash
make install INSTALL_DIR="$HOME/.local/bin"
```

`make install` 不会自动调用 `sudo`。如果 `/usr/local/bin` 不可写，先以当前用户构建，再只提升复制步骤的权限：

```bash
make build-release
sudo /usr/bin/install -v -m 0755 target/release/pm /usr/local/bin/pm
/usr/local/bin/pm --version
```

### 使用 Cargo 安装到 `~/.cargo/bin`：

```bash
cargo install --path .
```

## 数据存储

默认数据库路径为：

```text
$XDG_DATA_HOME/pm/pm.db
```

没有设置 `XDG_DATA_HOME` 时使用：

```text
$HOME/.local/share/pm/pm.db
```

## 创建和编辑

```bash
pm add code-review
pm edit code-review
pm rm code-review
pm rm code-review --force
```

`add` 和 `edit` 依次使用 `$VISUAL`、`$EDITOR` 和 `vi`。编辑器打开的 Markdown 格式为：

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

修改 `edit` 文档中的 `name` 会原子改名。编辑器失败、Markdown 无效或名称冲突时，原数据保持不变。

在交互式终端中，如果 Markdown 校验失败，`pm` 会显示错误并询问是否重新打开编辑器。选择重新编辑时会复用同一个临时文件，刚才的内容不会丢失；非 TTY 环境则直接失败。

`rm` 默认在 TTY 中要求确认；非交互调用必须传 `--force`。

## 获取 Prompt

`get` 获取并渲染 Prompt。Prompt 不包含变量或组合引用时，正文会原样输出：

```bash
pm get foobar
pm get --id 1
```

`get` 支持普通变量、文件变量和保留的 `input` 变量：

```bash
pm get foobar \
  -v language=rust \
  -v focus=correctness

pm get compare \
  --file old=old.rs \
  --file new=new.rs
```

安装外部 `fzf` 后，可以不提供名称并交互选择 Prompt：

```bash
pm get --pick
pm get --pick | codex exec -
```

`<NAME>`、`--id` 和 `--pick` 必须三选一。没有安装 `fzf` 或取消选择时返回错误。`fzf` 作为外部程序使用，不会打包进 `pm`。

变量名称必须匹配 `[a-zA-Z_][a-zA-Z0-9_-]*`。`--var` 与 `--file` 不能为同一个 key 同时提供值；显式提供的 `input` 优先于 stdin。缺少变量时命令失败，不会保留模板或替换为空。

只有模板包含 `{{input}}` 且没有显式提供 `input` 时，`get` 才读取 stdin。stdin 是交互式 TTY 时会立即失败，不会等待 EOF。

## 与 AI CLI 组合

`pm` 只负责生成 Prompt，可以把 stdout 直接交给任何能够从 stdin 读取 Prompt 的 AI CLI。

使用 Codex 非交互执行：

```bash
pm get prompt-name | codex exec -
```

使用 Claude Code 的 Print 模式：

```bash
pm get prompt-name | claude -p
```

也可以组合其他命令或重定向结果：

```bash
pm get prompt-name | another-ai-cli
some-command | pm get prompt-name | codex exec - > result.txt
```

其中 `codex exec -` 的 `-` 表示从 stdin 读取指令，`claude -p` 会输出响应后退出。`pm` 不读取这些工具的配置，也不绑定 Provider 或模型。

## 列表和搜索

```bash
pm list
pm list --tag coding
pm list --favorite
pm list --long --sort updated
pm list --long --sort used
pm search mongo
pm search mongo --name-only
```

`list` 按名称排序，默认格式为无表头 TSV：

```text
id<TAB>name
```

可以将其中的 ID 传给 `pm get --id ID`。Prompt 创建后，ID 不会因编辑或改名而变化，删除后也不会被新 Prompt 复用。

`search` 使用 SQLite FTS5 搜索 name、description 和 body，默认格式为 TSV：

```text
id<TAB>name<TAB>description
```

`--name-only` 每行只输出名称。包含多个词的查询使用字面量 AND 语义，不接受原始 FTS 表达式。

`list --tag` 使用 Tag 精确过滤。`--favorite` 只保留收藏项，两者可以组合。`--sort` 支持 `name`、`updated` 和 `used`；名称升序，时间倒序，未使用项排在最后。使用 `pm list --sort used` 可以按最近使用时间查看 Prompt。

`-l/--long` 增加时间列，时间使用 UTC RFC3339：

```text
id<TAB>name<TAB>updated_at<TAB>last_used_at
```

从收藏中添加或移除 Prompt：

```bash
pm favorite code-review
pm favorite code-review --remove
```

## Import 和 Export

```bash
pm export code-review > code-review.md
pm import code-review.md
pm export --all ./prompts/
```

`export --all` 按名称生成一个 Markdown 文件，并原子覆盖目录中的同名导出文件。Import 名称冲突时失败，不会覆盖已有 Prompt。Markdown 仅用于导入、导出、备份和分享；SQLite 始终是唯一数据源。

## Shell Completion

生成 bash、zsh 或 fish 的静态 completion：

```bash
pm completions bash > pm.bash
pm completions zsh > _pm
pm completions fish > pm.fish
```

加入 `--dynamic` 后，生成的脚本会在补全时只读访问 SQLite，为 `get`、`edit`、`rm`、`favorite` 和 `history` 补全 Prompt name：

```bash
pm completions bash --dynamic > pm.bash
pm completions zsh --dynamic > _pm
pm completions fish --dynamic > pm.fish
```

动态补全无法读取数据库或数据库忙时会安静地返回空候选，不会创建数据库或执行 migration。

## History

创建和每次有实际内容或 metadata 变化的编辑都会生成一个版本快照。无变化的编辑、`get` 和收藏操作不会生成版本。

```bash
pm history code-review
pm history code-review diff 1 3
```

`history` 输出无表头 TSV：

```text
version<TAB>created_at<TAB>historical_name
```

`history <NAME> diff <OLD> <NEW>` 输出包含 front matter 和 body 的 unified diff。Prompt 删除时，其历史版本也会随数据库记录一并删除。

## Prompt Composition

Prompt body 可以引用其他 Prompt：

```text
{{prompt:senior-engineer}}

{{prompt:security-guidelines}}

{{input}}
```

`get` 会递归展开组合引用，随后统一替换普通变量和 `input`。Export 仍输出原始引用。引用不存在或形成直接/间接循环时，`get` 失败且 stdout 保持为空。

## Pipeline 协议

- stdout 只包含 Prompt body、渲染结果、Markdown 或列表/搜索数据。
- errors、warnings 和确认提示只写入 stderr。
- 成功退出码为 `0`，运行时错误为 `1`，CLI 参数错误为 `2`。
- 输出管道提前关闭产生的 broken pipe 被视为正常结束。

## 开发

```bash
make fmt
make check
make build-release
```

项目当前聚焦 Prompt 的本地存储、搜索与文本转换，不包含 Provider、模型、API Key、聊天记录、Agent、TUI、daemon 或同步服务。
