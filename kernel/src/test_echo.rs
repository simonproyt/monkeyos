pub fn run() {
    let parts = vec!["echo", "donotcopy", "donotcopy"];
    let args = parts.into_iter().map(std::ffi::OsString::from);
    // Call uumain and let it write to stdout
    let _ = uu_echo::uumain(args);
}
