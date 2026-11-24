fn main() {
    if let Err(err) = c989ctl::gui::run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
