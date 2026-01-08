use std::fs;
use std::path::Path;

fn main() {
    // Generate build date and number
    generate_build_info();

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

fn generate_build_info() {
    // 只读取 .build_number 文件，不写入（由 build.ps1 管理）
    let build_file = Path::new(".build_number");
    let (date, build_num) = if build_file.exists() {
        let content = fs::read_to_string(build_file).unwrap_or_default();
        let parts: Vec<&str> = content.trim().split('_').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].parse::<u32>().unwrap_or(1))
        } else {
            ("unknown".to_string(), 1)
        }
    } else {
        ("unknown".to_string(), 1)
    };

    // Set environment variables for use in code
    println!("cargo:rustc-env=BUILD_DATE={}", date);
    println!("cargo:rustc-env=BUILD_NUMBER={}", build_num);
    println!("cargo:rustc-env=BUILD_INFO=build{}_{}", date, build_num);

    // 当 .build_number 文件变化时重新编译
    println!("cargo:rerun-if-changed=.build_number");
}
