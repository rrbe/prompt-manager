use std::{fs, path::Path};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn pm(data_home: &Path) -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("pm"));
    command
        .env("XDG_DATA_HOME", data_home)
        .env_remove("VISUAL")
        .env_remove("EDITOR");
    command
}

fn write_prompt(directory: &Path, file_name: &str, markdown: &str) -> std::path::PathBuf {
    let path = directory.join(file_name);
    fs::write(&path, markdown).unwrap();
    path
}

fn import_prompt(data_home: &Path, file_name: &str, markdown: &str) {
    let path = write_prompt(data_home, file_name, markdown);
    pm(data_home)
        .args(["import", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[cfg(unix)]
#[test]
fn add_opens_piped_stdin_as_the_initial_prompt_body() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    let snapshot = directory.path().join("editor-snapshot.md");
    let editor = directory.path().join("editor.sh");
    fs::write(
        &editor,
        "#!/bin/sh\ncp \"$1\" \"$EDITOR_SNAPSHOT\"\nprintf '%s' 'edited in editor' >> \"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o700)).unwrap();

    pm(directory.path())
        .env("VISUAL", &editor)
        .env("EDITOR_SNAPSHOT", &snapshot)
        .args(["add", "piped"])
        .write_stdin("first line\nsecond line\n")
        .assert()
        .success()
        .stdout("")
        .stderr("");
    assert!(
        fs::read_to_string(snapshot)
            .unwrap()
            .ends_with("first line\nsecond line\n")
    );

    pm(directory.path())
        .args(["get", "piped"])
        .assert()
        .success()
        .stdout("first line\nsecond line\nedited in editor")
        .stderr("");
}

#[cfg(unix)]
#[test]
fn add_opens_a_redirected_file_as_the_initial_prompt_body() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    let body = write_prompt(directory.path(), "body.txt", "redirected body\n");
    let input = fs::File::open(body).unwrap();
    let editor = directory.path().join("editor.sh");
    fs::write(
        &editor,
        "#!/bin/sh\nprintf '%s' 'edited in editor' >> \"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o700)).unwrap();
    let output = std::process::Command::new(assert_cmd::cargo::cargo_bin!("pm"))
        .env("XDG_DATA_HOME", directory.path())
        .env("VISUAL", &editor)
        .env_remove("EDITOR")
        .args(["add", "redirected"])
        .stdin(input)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");

    pm(directory.path())
        .args(["get", "redirected"])
        .assert()
        .success()
        .stdout("redirected body\nedited in editor")
        .stderr("");
}

#[cfg(unix)]
#[test]
fn add_no_edit_saves_stdin_without_opening_the_editor() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    let marker = directory.path().join("editor-opened");
    let editor = directory.path().join("editor.sh");
    fs::write(&editor, "#!/bin/sh\ntouch \"$EDITOR_MARKER\"\n").unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o700)).unwrap();

    pm(directory.path())
        .env("VISUAL", &editor)
        .env("EDITOR_MARKER", &marker)
        .args(["add", "direct", "--no-edit"])
        .write_stdin("direct body\n")
        .assert()
        .success()
        .stdout("")
        .stderr("");
    assert!(!marker.exists());

    pm(directory.path())
        .args(["get", "direct"])
        .assert()
        .success()
        .stdout("direct body\n")
        .stderr("");

    pm(directory.path())
        .env("VISUAL", &editor)
        .env("EDITOR_MARKER", &marker)
        .args(["add", "empty", "--no-edit"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
    assert!(!marker.exists());

    pm(directory.path())
        .args(["get", "empty"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[test]
fn add_no_edit_validates_the_prompt_body() {
    let directory = TempDir::new().unwrap();

    pm(directory.path())
        .args(["add", "invalid-template", "--no-edit"])
        .write_stdin("{{ daily report content }}")
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "invalid template syntax at line 5, column 1: invalid expression `{{ daily report content }}`",
        ));
}

#[test]
fn renders_piped_input_and_explicit_variables_with_clean_stdout() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "review.md",
        "---\nname: review\n---\n\nLanguage: {{language}}\n\n{{input}}",
    );

    pm(directory.path())
        .args(["get", "review", "-v", "language=rust"])
        .write_stdin("hello\n")
        .assert()
        .success()
        .stdout("Language: rust\n\nhello\n")
        .stderr("");
}

#[test]
fn gets_plain_prompt_without_consuming_piped_input() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "plain.md",
        "---\nname: plain\n---\n\nplain body",
    );

    pm(directory.path())
        .args(["get", "plain"])
        .write_stdin("ignored")
        .assert()
        .success()
        .stdout("plain body")
        .stderr("");
}

#[test]
fn gets_prompt_by_id() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "by-id.md",
        "---\nname: by-id\n---\n\nselected by id",
    );

    pm(directory.path())
        .args(["get", "--id", "1"])
        .assert()
        .success()
        .stdout("selected by id")
        .stderr("");
}

#[test]
fn explicit_input_overrides_piped_input() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "input.md",
        "---\nname: input-test\n---\n\n{{input}}",
    );

    pm(directory.path())
        .args(["get", "input-test", "-v", "input=explicit"])
        .write_stdin("piped")
        .assert()
        .success()
        .stdout("explicit")
        .stderr("");
}

#[test]
fn missing_variables_fail_without_stdout() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "missing.md",
        "---\nname: missing\n---\n\n{{language}}",
    );

    pm(directory.path())
        .args(["get", "missing"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("missing variable: language"));
}

#[test]
fn get_uses_variable_defaults_and_explicit_values_override_them() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "defaults.md",
        "---\nname: defaults\n---\n\n{{language=rust}}/{{language}}/{{query=a=b}}",
    );

    pm(directory.path())
        .args(["get", "defaults"])
        .assert()
        .success()
        .stdout("rust/rust/a=b")
        .stderr("");

    pm(directory.path())
        .args(["get", "defaults", "-v", "language=", "-v", "query=override"])
        .assert()
        .success()
        .stdout("//override")
        .stderr("");
}

#[cfg(unix)]
#[test]
fn exec_renders_stdin_and_variables_and_appends_arguments() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    let executable = directory.path().join("capture-exec.sh");
    fs::write(
        &executable,
        "#!/bin/sh\nfor argument in \"$@\"; do printf 'arg=<%s>\\n' \"$argument\"; done\nprintf 'prompt:\\n'\ncat\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    import_prompt(
        directory.path(),
        "executable.md",
        &format!(
            "---\nname: executable\nexec: {} \"configured value\" '$HOME'\n---\n\nLanguage: {{{{language}}}}\n\n{{{{input}}}}",
            executable.display()
        ),
    );

    pm(directory.path())
        .args([
            "exec",
            "executable",
            "-v",
            "language=rust",
            "--",
            "--model",
            "gpt-5.4",
        ])
        .write_stdin("source\n")
        .assert()
        .success()
        .stdout(
            "arg=<configured value>\narg=<$HOME>\narg=<--model>\narg=<gpt-5.4>\nprompt:\nLanguage: rust\n\nsource\n",
        )
        .stderr("");

    pm(directory.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("now"))
        .stderr("");
}

#[cfg(unix)]
#[test]
fn exec_supports_id_and_propagates_the_command_exit_code() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    let executable = directory.path().join("failing-exec.sh");
    fs::write(&executable, "#!/bin/sh\ncat >/dev/null\nexit 7\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    import_prompt(
        directory.path(),
        "failing.md",
        &format!(
            "---\nname: failing\nexec: {}\n---\n\nprompt body",
            executable.display()
        ),
    );

    pm(directory.path())
        .args(["exec", "--id", "1"])
        .assert()
        .failure()
        .code(7)
        .stdout("")
        .stderr("");

    pm(directory.path())
        .args(["list", "--sort", "used"])
        .assert()
        .success()
        .stdout(predicate::str::contains("now"))
        .stderr("");
}

#[test]
fn exec_requires_a_configured_command_and_reports_start_failures() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "plain-exec.md",
        "---\nname: plain-exec\n---\n\nbody",
    );
    import_prompt(
        directory.path(),
        "missing-program.md",
        "---\nname: missing-program\nexec: pm-program-that-does-not-exist\n---\n\nbody",
    );

    pm(directory.path())
        .args(["exec", "plain-exec"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "prompt has no exec command: plain-exec",
        ));
    pm(directory.path())
        .args(["exec", "missing-program"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "failed to start exec command `pm-program-that-does-not-exist`",
        ));
    pm(directory.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("now").not())
        .stderr("");
}

#[test]
fn interactive_get_prompts_for_missing_variables_and_keeps_stdout_clean() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "interactive-shared.md",
        "---\nname: interactive-shared\n---\n\nShared {{ details }}",
    );
    import_prompt(
        directory.path(),
        "interactive.md",
        "---\nname: interactive\n---\n\n{{ provided }}\n{{ prompt:interactive-shared }}\n{{ provided }}",
    );

    pm(directory.path())
        .args([
            "get",
            "interactive",
            "--interactive",
            "-v",
            "provided=fixed",
        ])
        .write_stdin("\nfirst line\nsecond line\n\nEOF\n")
        .assert()
        .success()
        .stdout("fixed\nShared first line\nsecond line\nfixed")
        .stderr(
            "[1/1] details\nEnter or paste the value. Finish with a line containing only `EOF`, or press Ctrl-D on an empty line.\n",
        );
}

#[test]
fn interactive_get_fails_without_a_completed_value() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "interactive-incomplete.md",
        "---\nname: interactive-incomplete\n---\n\n{{details}}",
    );

    pm(directory.path())
        .args(["get", "interactive-incomplete", "--interactive"])
        .write_stdin("unfinished\n")
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "interactive input ended before completing variable: details",
        ));
}

#[test]
fn non_tty_input_fulfills_input_even_when_empty() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "empty.md",
        "---\nname: empty-input\n---\n\nBefore{{input}}After",
    );

    pm(directory.path())
        .args(["get", "empty-input"])
        .write_stdin("")
        .assert()
        .success()
        .stdout("BeforeAfter")
        .stderr("");
}

#[test]
fn piped_input_overrides_its_default_including_when_empty() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "default-input.md",
        "---\nname: default-input\n---\n\n{{input=fallback}}",
    );

    pm(directory.path())
        .args(["get", "default-input"])
        .write_stdin("piped")
        .assert()
        .success()
        .stdout("piped")
        .stderr("");

    pm(directory.path())
        .args(["get", "default-input"])
        .write_stdin("")
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[test]
fn removed_file_option_is_rejected_and_duplicate_variables_fail() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "variable.md",
        "---\nname: variable-test\n---\n\n{{source}}",
    );

    pm(directory.path())
        .args(["get", "variable-test", "--file", "source=source.txt"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("unexpected argument '--file'"));

    pm(directory.path())
        .args([
            "get",
            "variable-test",
            "-v",
            "source=first",
            "-v",
            "source=second",
        ])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "duplicate variable source: source",
        ));
}

#[test]
fn export_remove_import_round_trip_preserves_content_tags_and_exec() {
    let directory = TempDir::new().unwrap();
    let original = "---\nname: round-trip\ndescription: A test\ntags:\n  - zeta\n  - alpha\nexec: codex exec -\n---\n\nBody\n\n{{input}}";
    import_prompt(directory.path(), "original.md", original);

    let exported = pm(directory.path())
        .args(["export", "round-trip"])
        .output()
        .unwrap();
    assert!(exported.status.success());
    assert!(exported.stderr.is_empty());
    let exported_markdown = String::from_utf8(exported.stdout).unwrap();
    let exported_path = directory.path().join("exported.md");
    fs::write(&exported_path, &exported_markdown).unwrap();

    pm(directory.path())
        .args(["rm", "round-trip", "--force"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
    pm(directory.path())
        .args(["import", exported_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");
    pm(directory.path())
        .args(["export", "round-trip"])
        .assert()
        .success()
        .stdout(exported_markdown)
        .stderr("");
}

#[test]
fn import_rejects_invalid_template_syntax() {
    let directory = TempDir::new().unwrap();
    let path = write_prompt(
        directory.path(),
        "invalid-template.md",
        "---\nname: invalid-template\n---\n\n{{ daily report content }}",
    );

    pm(directory.path())
        .args(["import", path.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "invalid template syntax at line 5, column 1: invalid expression `{{ daily report content }}`",
        ));

    let conflicting = write_prompt(
        directory.path(),
        "conflicting-defaults.md",
        "---\nname: conflicting-defaults\n---\n\n{{language=rust}}\n{{language=go}}",
    );
    pm(directory.path())
        .args(["import", conflicting.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "invalid template syntax at line 6, column 1: conflicting defaults for variable `language`",
        ));
}

#[test]
fn import_rejects_invalid_exec_commands() {
    let directory = TempDir::new().unwrap();
    let empty = write_prompt(
        directory.path(),
        "empty-exec.md",
        "---\nname: empty-exec\nexec: '  '\n---\n\nbody",
    );
    pm(directory.path())
        .args(["import", empty.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("exec command must not be empty"));

    let null = write_prompt(
        directory.path(),
        "null-exec.md",
        "---\nname: null-exec\nexec:\n---\n\nbody",
    );
    pm(directory.path())
        .args(["import", null.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("exec command must not be empty"));

    let invalid = write_prompt(
        directory.path(),
        "invalid-exec.md",
        "---\nname: invalid-exec\nexec: \"codex '\"\n---\n\nbody",
    );
    pm(directory.path())
        .args(["import", invalid.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("invalid exec command"));
}

#[test]
fn list_renders_a_table_and_search_has_a_stable_line_format() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "beta.md",
        "---\nname: beta\ndescription: Mongo\tcheck\ntags:\n  - database\nexec: codex exec -\n---\n\nOther body",
    );
    import_prompt(
        directory.path(),
        "alpha.md",
        "---\nname: alpha\ndescription: First\ntags:\n  - coding\n---\n\nMongo keyword",
    );

    pm(directory.path())
        .arg("list")
        .env("PAGER", "false")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(
                "^ID  NAME   USES  LAST USE\n──  ─────  ─{4}  ─{8}\n2   alpha  0     -\n1   beta   0     -\n$",
            )
            .unwrap(),
        )
        .stderr("");
    pm(directory.path())
        .args(["list", "--tag", "coding"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID  NAME   USES"))
        .stdout(predicate::str::contains("\n2   alpha  "))
        .stderr("");
    pm(directory.path())
        .args(["list", "-q"])
        .env("CLICOLOR_FORCE", "1")
        .env("PAGER", "false")
        .assert()
        .success()
        .stdout("alpha\nbeta\n")
        .stderr("");
    pm(directory.path())
        .args(["list", "--full"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID  NAME   TAGS"))
        .stdout(predicate::str::contains("FAVORITE  USES  CREATED AT"))
        .stdout(predicate::str::contains("coding"))
        .stdout(predicate::str::contains("database"))
        .stdout(predicate::str::contains("codex exec -"))
        .stdout(predicate::str::contains("Mongo check"))
        .stderr("");
    pm(directory.path())
        .args(["list", "--full", "--quiet"])
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("cannot be used with"));
    pm(directory.path())
        .args(["search", "Mongo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2\talpha\tFirst\n"))
        .stdout(predicate::str::contains("1\tbeta\tMongo check\n"))
        .stderr("");
    pm(directory.path())
        .args(["search", "Mongo", "--name-only"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "unexpected argument '--name-only'",
        ));
}

#[test]
fn list_help_explains_default_full_and_quiet_output() {
    let directory = TempDir::new().unwrap();

    pm(directory.path())
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show all prompt metadata except content",
        ))
        .stdout(predicate::str::contains("Print only prompt names"))
        .stdout(predicate::str::contains("pm list --quiet | fzf"))
        .stderr("");
}

#[test]
fn list_filters_prompts_by_group_prefix() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "weekly.md",
        "---\nname: work/week-report\n---\n\nweekly",
    );
    import_prompt(
        directory.path(),
        "monthly.md",
        "---\nname: work/month-report\n---\n\nmonthly",
    );
    import_prompt(
        directory.path(),
        "personal.md",
        "---\nname: personal/week-report\n---\n\npersonal",
    );
    import_prompt(
        directory.path(),
        "workbench.md",
        "---\nname: workbench/report\n---\n\nworkbench",
    );

    pm(directory.path())
        .args(["list", "work/"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work/month-report"))
        .stdout(predicate::str::contains("work/week-report"))
        .stdout(predicate::str::contains("personal/week-report").not())
        .stdout(predicate::str::contains("workbench/report").not())
        .stderr("");

    pm(directory.path())
        .args(["list", "work"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "prompt group must end with '/': work",
        ));
}

#[test]
fn list_supports_sorting_and_favorite_filters() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "alpha-list.md",
        "---\nname: alpha-list\ntags:\n  - coding\n---\n\nbody",
    );
    import_prompt(
        directory.path(),
        "beta-list.md",
        "---\nname: beta-list\ntags:\n  - coding\n---\n\nbody",
    );
    pm(directory.path())
        .args(["get", "beta-list"])
        .assert()
        .success();
    pm(directory.path())
        .args(["favorite", "beta-list"])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    pm(directory.path())
        .args(["list", "--tag", "coding", "--favorite"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\n2   beta-list  "))
        .stderr("");
    pm(directory.path())
        .args(["list", "--sort", "used"])
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(
                "^ID  NAME        USES  LAST USE\n[^\n]*\n2   beta-list   1     (now|[^\n]+ ago)\n1   alpha-list  0     -\n$",
            )
            .unwrap(),
        )
        .stderr("");
    pm(directory.path())
        .args(["list", "-r"])
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(
                "^ID  NAME        USES  LAST USE\n[^\n]*\n1   alpha-list  0     -\n2   beta-list   1     (now|[^\n]+ ago)\n$",
            )
            .unwrap(),
        )
        .stderr("");

    pm(directory.path())
        .args(["favorite", "beta-list", "--remove"])
        .assert()
        .success();
    pm(directory.path())
        .args(["list", "--favorite"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[test]
fn export_all_writes_one_markdown_file_per_prompt() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "one.md",
        "---\nname: export-one\n---\n\none",
    );
    import_prompt(
        directory.path(),
        "two.md",
        "---\nname: export-two\n---\n\ntwo",
    );
    import_prompt(
        directory.path(),
        "grouped.md",
        "---\nname: work/week-report\n---\n\ngrouped",
    );
    let output = directory.path().join("backup");

    for _ in 0..2 {
        pm(directory.path())
            .args(["export", "--all", output.to_str().unwrap()])
            .assert()
            .success()
            .stdout("")
            .stderr("");
    }
    assert!(
        fs::read_to_string(output.join("export-one.md"))
            .unwrap()
            .ends_with("\n\none")
    );
    assert!(
        fs::read_to_string(output.join("export-two.md"))
            .unwrap()
            .ends_with("\n\ntwo")
    );
    assert!(
        fs::read_to_string(output.join("work/week-report.md"))
            .unwrap()
            .ends_with("\n\ngrouped")
    );
}

#[cfg(unix)]
#[test]
fn history_and_diff_include_atomic_edit_versions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "history.md",
        "---\nname: versioned\ntags:\n  - first\n---\n\nversion one",
    );
    let editor = directory.path().join("history-editor.sh");
    fs::write(
        &editor,
        "#!/bin/sh\nprintf '%s' '---\nname: versioned\ntags:\n  - second\nexec: codex exec -\n---\n\nversion two' > \"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o700)).unwrap();
    pm(directory.path())
        .env("VISUAL", &editor)
        .args(["edit", "versioned"])
        .assert()
        .success();

    pm(directory.path())
        .args(["history", "versioned"])
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(
                "^VERSION  CREATED AT +NAME\\n─{7}  ─{16}  ─{9}\\n2 +[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}  versioned\\n1 +[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}  versioned\\n$",
            )
            .unwrap(),
        )
        .stderr("");
    pm(directory.path())
        .args(["history", "versioned", "diff", "1", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--- versioned@1"))
        .stdout(predicate::str::contains("+++ versioned@2"))
        .stdout(predicate::str::contains("-version one"))
        .stdout(predicate::str::contains("+version two"))
        .stdout(predicate::str::contains("+exec: codex exec -"))
        .stderr("");
}

#[test]
fn get_expands_nested_prompt_composition_and_rejects_cycles() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "rules.md",
        "---\nname: shared-rules\n---\n\nRules for {{language}}.",
    );
    import_prompt(
        directory.path(),
        "middle.md",
        "---\nname: middle\n---\n\n{{prompt:shared-rules}}",
    );
    import_prompt(
        directory.path(),
        "composed.md",
        "---\nname: composed\n---\n\nStart\n{{prompt:middle}}\n{{input}}",
    );
    pm(directory.path())
        .args(["get", "composed", "-v", "language=rust"])
        .write_stdin("code")
        .assert()
        .success()
        .stdout("Start\nRules for rust.\ncode")
        .stderr("");

    import_prompt(
        directory.path(),
        "cycle-a.md",
        "---\nname: cycle-a\n---\n\n{{prompt:cycle-b}}",
    );
    import_prompt(
        directory.path(),
        "cycle-b.md",
        "---\nname: cycle-b\n---\n\n{{prompt:cycle-a}}",
    );
    pm(directory.path())
        .args(["get", "cycle-a"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "prompt composition cycle: cycle-a -> cycle-b -> cycle-a",
        ));
}

#[test]
fn dynamic_completion_reads_prompt_names_from_the_database() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "dynamic.md",
        "---\nname: dynamic-name\n---\n\nbody",
    );

    pm(directory.path())
        .env("COMPLETE", "zsh")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .env("_CLAP_IFS", "\n")
        .args(["--", "pm", "get", "dynamic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dynamic-name"))
        .stderr("");
    pm(directory.path())
        .env("COMPLETE", "zsh")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .env("_CLAP_IFS", "\n")
        .args(["--", "pm", "exec", "dynamic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dynamic-name"))
        .stderr("");
    pm(directory.path())
        .args(["completions", "zsh", "--dynamic"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_clap_dynamic_completer_pm"))
        .stderr("");
}

#[test]
fn get_help_shows_codex_usage_examples() {
    let directory = TempDir::new().unwrap();
    pm(directory.path())
        .args(["get", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("codex \"$(pm get prompt-name)\"").and(
                predicate::str::contains("pm get prompt-name | codex exec -"),
            ),
        )
        .stderr("");
}

#[test]
fn exec_help_shows_configured_command_and_appended_argument_examples() {
    let directory = TempDir::new().unwrap();
    pm(directory.path())
        .args(["exec", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("pm exec prompt-name").and(predicate::str::contains(
                "pm exec prompt-name -- --model gpt-5.4",
            )),
        )
        .stderr("");
}

#[test]
fn completions_help_explains_generation_and_installation() {
    let directory = TempDir::new().unwrap();
    pm(directory.path())
        .args(["completions", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("written to stdout")
                .and(predicate::str::contains("pm completions zsh --dynamic"))
                .and(predicate::str::contains(
                    "~/.local/share/bash-completion/completions/pm",
                ))
                .and(predicate::str::contains(
                    "~/.config/fish/completions/pm.fish",
                )),
        )
        .stderr("");
}

#[test]
fn completions_do_not_open_the_database() {
    let directory = TempDir::new().unwrap();
    pm(directory.path())
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_pm"))
        .stderr("");
    pm(directory.path())
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c pm"))
        .stderr("");
    assert!(!directory.path().join("pm/pm.db").exists());
}

#[cfg(unix)]
#[test]
fn get_pick_renders_the_prompt_selected_by_external_fzf() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "alpha-pick.md",
        "---\nname: alpha-pick\n---\n\nalpha body",
    );
    import_prompt(
        directory.path(),
        "beta-pick.md",
        "---\nname: beta-pick\n---\n\n{{language}} beta body",
    );
    let binary_directory = directory.path().join("bin");
    fs::create_dir(&binary_directory).unwrap();
    let fzf = binary_directory.join("fzf");
    fs::write(&fzf, "#!/bin/sh\nsed -n '2p'\n").unwrap();
    fs::set_permissions(&fzf, fs::Permissions::from_mode(0o700)).unwrap();
    let path = format!(
        "{}:{}",
        binary_directory.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    pm(directory.path())
        .env("PATH", path)
        .args(["get", "--pick", "-v", "language=rust"])
        .assert()
        .success()
        .stdout("rust beta body")
        .stderr("");
}

#[test]
fn get_pick_reports_a_missing_fzf() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "pick.md",
        "---\nname: pick-me\n---\n\nbody",
    );

    pm(directory.path())
        .env("PATH", "")
        .args(["get", "--pick"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("fzf is not installed"));
}

#[test]
fn missing_prompt_and_unforced_non_tty_remove_fail_cleanly() {
    let directory = TempDir::new().unwrap();
    pm(directory.path())
        .args(["get", "does-not-exist"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("prompt not found: does-not-exist"));

    import_prompt(
        directory.path(),
        "remove.md",
        "---\nname: remove-me\n---\n\nbody",
    );
    pm(directory.path())
        .args(["rm", "remove-me"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("use --force"));
    pm(directory.path())
        .args(["get", "remove-me"])
        .assert()
        .success()
        .stdout("body")
        .stderr("");
}

#[test]
fn remove_alias_is_visible_and_removes_a_prompt() {
    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "remove-alias.md",
        "---\nname: remove-alias\n---\n\nbody",
    );

    pm(directory.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("alias: remove"))
        .stderr("");
    pm(directory.path())
        .args(["remove", "remove-alias", "--force"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
    pm(directory.path())
        .args(["get", "remove-alias"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("prompt not found: remove-alias"));
}

#[cfg(unix)]
#[test]
fn add_rejects_an_existing_name_before_opening_the_editor() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "existing.md",
        "---\nname: existing\n---\n\nbody",
    );

    let marker = directory.path().join("editor-opened");
    let editor = directory.path().join("editor.sh");
    fs::write(&editor, "#!/bin/sh\ntouch \"$EDITOR_MARKER\"\n").unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o700)).unwrap();

    pm(directory.path())
        .env("VISUAL", &editor)
        .env("EDITOR_MARKER", &marker)
        .args(["add", "existing"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("prompt already exists: existing"));
    assert!(!marker.exists());
}

#[cfg(unix)]
#[test]
fn edit_uses_an_external_editor_and_protects_the_original_on_failure() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new().unwrap();
    import_prompt(
        directory.path(),
        "created.md",
        "---\nname: created\ntags:\n  - test\n---\n\nfirst body",
    );
    let editor = directory.path().join("editor.sh");
    fs::write(&editor, "#!/bin/sh\nexit 7\n").unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o700)).unwrap();

    pm(directory.path())
        .env("VISUAL", &editor)
        .args(["edit", "created"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("editor exited with status"));
    pm(directory.path())
        .args(["get", "created"])
        .assert()
        .success()
        .stdout("first body");

    fs::write(
        &editor,
        "#!/bin/sh\nprintf '%s' '---\nname: created\n---\n\n{{ daily report content }}' > \"$1\"\n",
    )
    .unwrap();
    pm(directory.path())
        .env("VISUAL", &editor)
        .args(["edit", "created"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "invalid template syntax at line 5, column 1: invalid expression `{{ daily report content }}`",
        ));
    pm(directory.path())
        .args(["get", "created"])
        .assert()
        .success()
        .stdout("first body");

    fs::write(
        &editor,
        "#!/bin/sh\nprintf '%s' '---\n---\n\ninvalid' > \"$1\"\n",
    )
    .unwrap();
    pm(directory.path())
        .env("VISUAL", &editor)
        .args(["edit", "created"])
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("invalid YAML front matter"));
    pm(directory.path())
        .args(["get", "created"])
        .assert()
        .success()
        .stdout("first body");
}

#[test]
fn invalid_cli_arguments_use_exit_code_two() {
    let directory = TempDir::new().unwrap();
    pm(directory.path())
        .args(["get", "test", "-v", "invalid"])
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("expected KEY=VALUE"));

    pm(directory.path())
        .args(["get", "test", "--pick"])
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("cannot be used with"));
}
