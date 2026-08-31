fn main() {
    if let Err(error) = prompt_manager::run() {
        if error.is_broken_pipe() {
            return;
        }
        if let Some(code) = error.exec_exit_code() {
            std::process::exit(code);
        }

        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
