use std::fs;
use std::path::Path;

fn main() {
    // Generate build date and number (完美保留原有的版本追踪逻辑)
    generate_build_info();

    #[cfg(windows)]
    {
        //  assets 文件夹里找你的专属
        if std::path::Path::new("assets/exe_icon.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/exe_icon.ico");
            // 核心属性修改区：这里就是鼠标悬停时会显示的信息！
            res.set("FileDescription", "STS 3.0 - MionaRira Edition"); // 文件说明
            res.set("ProductName", "STS MionaRira Edition");                     // 产品名称
            res.set("OriginalFilename", "STS_MionaRira.exe");                    // 原始文件名
            res.set("LegalCopyright", "Copyright (c) 2026");  // 版权信息
            
            // 执行嵌入
            res.compile().unwrap();
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