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
    use std::time::SystemTime;

    // Get current date in YYYYMMDD format
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Convert to YYYYMMDD format
    let current_date = format_date_yyyymmdd(now);

    // Read or create build number file
    let build_file = Path::new(".build_number");
    let (stored_date, build_num) = if build_file.exists() {
        let content = fs::read_to_string(build_file).unwrap_or_default();
        let parts: Vec<&str> = content.trim().split('_').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].parse::<u32>().unwrap_or(0))
        } else {
            (String::new(), 0)
        }
    } else {
        (String::new(), 0)
    };

    // Increment build number if same date, otherwise reset to 1
    let new_build_num = if stored_date == current_date {
        build_num + 1
    } else {
        1
    };

    // Write new build info
    let build_info = format!("{}_{}", current_date, new_build_num);
    fs::write(build_file, &build_info).ok();

    // Set environment variables for use in code
    println!("cargo:rustc-env=BUILD_DATE={}", current_date);
    println!("cargo:rustc-env=BUILD_NUMBER={}", new_build_num);
    println!("cargo:rustc-env=BUILD_INFO=build{}", build_info);
}

fn format_date_yyyymmdd(timestamp: u64) -> String {
    // Convert Unix timestamp to YYYYMMDD
    // This is a simplified calculation
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_PER_YEAR: u64 = 365;
    const DAYS_PER_LEAP_YEAR: u64 = 366;

    let mut days = timestamp / SECONDS_PER_DAY;
    let mut year = 1970;

    // Find the year
    loop {
        let days_in_year = if is_leap_year(year) { DAYS_PER_LEAP_YEAR } else { DAYS_PER_YEAR };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    // Find the month and day
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    let mut day = days + 1;

    for &days_in_month in &days_in_months {
        if day <= days_in_month as u64 {
            break;
        }
        day -= days_in_month as u64;
        month += 1;
    }

    format!("{:04}{:02}{:02}", year, month, day)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
