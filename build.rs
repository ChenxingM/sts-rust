fn main() {
    #[cfg(windows)]
    {
        // Only add icon if the file exists
        if std::path::Path::new("icon.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("icon.ico");
            res.compile().expect("Failed to compile Windows resources");
        }
    }
}
