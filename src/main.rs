fn main() {
    if let Err(error) = prompt_manager::run() {
        if error.is_broken_pipe() {
            return;
        }
        if let Some(code) = error.exec_exit_code() {
            std::process::exit(code);
        }

        if matches!(error, prompt_manager::error::Error::LintFailed) {
            std::process::exit(1);
        }

        const ERROR_STYLE: anstyle::Style = anstyle::AnsiColor::Red.on_default().bold();
        anstream::eprintln!("{ERROR_STYLE}Error{ERROR_STYLE:#}  {error}");
        std::process::exit(1);
    }
}
