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
pm get code-review | claude -p
codex "$(pm get code-review)"
pi "$(pm get code-review)"
pm get code-review | pbcopy # macOS 复制到剪贴板

# 如果设置了“执行方式”（类似 systemd 的 ExecStart），可以直接执行
pm exec code-review

# 编辑/删除提示词
pm edit code-review
pm rm code-review
```

## 安装

### 安装脚本（macOS / Linux）

安装最新版本。需要 `curl`，以及 `shasum` 或 `sha256sum`。脚本自动识别平台、校验二进制文件的 SHA256，并设置执行权限。

```bash
curl -fsSL https://github.com/rrbe/prompt-manager/releases/latest/download/install.sh | bash
```

默认安装到 `/usr/local/bin`。如果没有管理员权限，可以安装到用户目录，并将其加入 `PATH`：

```bash
curl -fsSL https://github.com/rrbe/prompt-manager/releases/latest/download/install.sh | PM_INSTALL_DIR="$HOME/.local/bin" bash
export PATH="$HOME/.local/bin:$PATH"
```

### 使用 wget 下载（macOS / Linux）

从 [Releases](https://github.com/rrbe/prompt-manager/releases) 选择版本，并选择对应平台：

| 平台 | 值 |
| --- | --- |
| macOS，Apple Silicon | `apple-arm` |
| macOS，Intel | `apple-intel` |
| Linux，ARM64 | `linux-arm` |
| Linux，x86_64（Intel 或 AMD） | `linux-intel` |

将 `vX.Y.Z` 替换为发布标签，并根据机器设置 `PLATFORM`：

```bash
VERSION=vX.Y.Z
PLATFORM=apple-arm
BINARY="pm-${VERSION}-${PLATFORM}"
BASE="https://github.com/rrbe/prompt-manager/releases/download/${VERSION}"

wget "${BASE}/${BINARY}" "${BASE}/${BINARY}.sha256"
# macOS：
shasum -a 256 -c "${BINARY}.sha256"
# Linux：改用 sha256sum -c "${BINARY}.sha256"
```

确认校验结果为 `OK` 后安装：

```bash
mkdir -p "$HOME/.local/bin"
install -m 0755 "$BINARY" "$HOME/.local/bin/pm"
export PATH="$HOME/.local/bin:$PATH"
pm --version
```

安装到系统目录时，将安装命令改为 `sudo install -m 0755 "$BINARY" /usr/local/bin/pm`。

### Windows

x86_64（Intel 或 AMD）下载 `pm-vX.Y.Z-windows-intel.exe`，ARM64 下载 `pm-vX.Y.Z-windows-arm.exe`。例如在 PowerShell 中，将 `vX.Y.Z` 替换为发布标签并选择平台：

```powershell
$ErrorActionPreference = "Stop"
$Version = "vX.Y.Z"
$Platform = "windows-intel" # ARM64 使用 windows-arm
$Binary = "pm-$Version-$Platform.exe"
$Base = "https://github.com/rrbe/prompt-manager/releases/download/$Version"
Invoke-WebRequest "$Base/$Binary" -OutFile $Binary
Invoke-WebRequest "$Base/$Binary.sha256" -OutFile "$Binary.sha256"
if ((Get-FileHash $Binary -Algorithm SHA256).Hash -ne (Get-Content "$Binary.sha256").Split()[0]) {
    throw "SHA256 checksum mismatch"
}
$InstallDir = "$env:LOCALAPPDATA\Programs\pm"
New-Item -ItemType Directory -Force $InstallDir | Out-Null
Move-Item $Binary "$InstallDir\pm.exe" -Force
$env:Path = "$InstallDir;$env:Path"
pm --version
```

将 `%LOCALAPPDATA%\Programs\pm` 加入用户环境变量 `Path`，即可在之后打开的终端中使用 `pm`。交互编辑默认使用记事本；可通过 `VISUAL` 或 `EDITOR` 设置其他编辑器，例如 `code --wait`。

安装脚本和各平台二进制文件从 v0.7.0 之后的版本开始提供。v0.7.0 及更早版本的 `pm update` 使用旧文件名，需要按上述方式重新安装一次。

### 从源码安装

安装 Rust 后执行：

```bash
cargo install --git https://github.com/rrbe/prompt-manager --locked
```

在本地源码目录中也可以执行：

```bash
cargo install --path . --locked               # 安装到 Cargo 的 bin 目录
# macOS / Linux：
make install                                  # 默认安装到 /usr/local/bin
make install INSTALL_DIR="$HOME/.local/bin"   # 安装到用户目录
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

macOS/Linux 的数据库默认存储在 `$HOME/.local/share/pm/pm.db`，Windows 默认存储在 `%LOCALAPPDATA%\pm\pm.db`。设置 `XDG_DATA_HOME` 后，则使用该目录下的 `pm/pm.db`。

## 开发

```bash
make fmt
make check
make build-release
```
