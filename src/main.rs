fn main() {
    if let Err(error) = prompt_manager::run() {
        if error.is_broken_pipe() {
            return;
        }

        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
