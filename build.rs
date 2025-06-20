use std::fs::File;
use std::io::Write;
use std::process;

fn log_to_file(message: &str) {
    let mut file = File::options()
        .append(true) // Enable appending
        .create(true) // Create the file if it doesn't exist
        .open("build_debug.log") // Open the log file
        .expect("Unable to open or create debug log file");
    writeln!(file, "{}", message).expect("Unable to write to debug log file");
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=resources/app_icon.ico");
        println!("cargo:rerun-if-changed=build.rs");
        println!("Running build.rs........");

        log_to_file("Running build.rs...");

        let icon_path = "resources/app_icon.ico";

        if !std::path::Path::new(icon_path).exists() {
            eprintln!("Icon file not found: {}", icon_path);
            process::exit(1);
        }

        log_to_file(&format!("Using icon path: {}", icon_path));

        let mut res = winres::WindowsResource::new();
        res.set_icon(icon_path);

        match res.compile() {
            Ok(_) => {
                log_to_file("Resource compiled successfully.");
            }
            Err(e) => {
                log_to_file(&format!("Failed to compile resources: {}", e));
                panic!("Failed to embed resources into binary!");
            }
        }

        log_to_file("Finished running build.rs");
    }
}
