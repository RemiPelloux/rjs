use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;

const ITERATIONS: usize = 3;

// Use a static variable to store test directory path
static CURRENT_TEST_DIR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

fn measure_command(cmd: &[&str]) -> (Duration, String, bool) {
    let start = Instant::now();

    // Get the path to the binary built in release mode using an environment variable 
    // for the project directory or resolving to parent of test dir
    let project_dir = match std::env::var("RJS_PROJECT_DIR") {
        Ok(dir) => Path::new(&dir).to_path_buf(),
        Err(_) => {
            // Fallback: find project directory by looking for Cargo.toml
            let mut current = std::env::current_dir()
                .expect("Failed to get current directory");
            
            // Go up until we find Cargo.toml or hit the root
            while !current.join("Cargo.toml").exists() {
                if !current.pop() {
                    panic!("Could not find Cargo.toml in any parent directory");
                }
            }
            current
        }
    };

    let bin_path = project_dir.join("target/release/rjs");

    // Ensure the binary exists
    if !bin_path.exists() {
        panic!("Binary not found at {}", bin_path.display());
    }

    let output = Command::new(bin_path)
        .args(cmd)
        .output()
        .expect("Failed to execute command");

    let elapsed = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();

    (elapsed, stdout, success)
}

fn setup() {
    // Get the temp dir from environment variable or use a default
    let test_dir_str = std::env::var("RJS_TEST_TEMP_DIR")
        .unwrap_or_else(|_| "test_project".to_string());
    let test_dir = Path::new(&test_dir_str);
    
    // Create a subdirectory for this specific test run
    let run_id = format!("test_run_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs());
    let run_dir = test_dir.join(run_id);
    
    // Create the test directory if it doesn't exist
    if !run_dir.exists() {
        fs::create_dir_all(&run_dir).expect("Failed to create test directory");
    }

    // Change to the test directory
    std::env::set_current_dir(&run_dir).expect("Failed to change directory");
    
    // Store the path for cleanup using the static mutex
    if let Ok(mut current_dir) = CURRENT_TEST_DIR.lock() {
        *current_dir = Some(run_dir.to_string_lossy().to_string());
    }
}

fn cleanup() {
    // Get the current test directory from static mutex
    if let Ok(mut current_dir) = CURRENT_TEST_DIR.lock() {
        if let Some(test_dir) = current_dir.take() {
            // Change back to the parent directory
            if let Some(parent) = Path::new(&test_dir).parent() {
                if let Err(e) = std::env::set_current_dir(parent) {
                    eprintln!("Warning: Failed to change directory: {}", e);
                }
            }

            // Remove the test directory
            if let Err(e) = fs::remove_dir_all(&test_dir) {
                eprintln!("Warning: Failed to remove test directory: {}", e);
            }
        }
    }
}

fn print_result(command: &str, duration: Duration, success: bool, stdout: &str) {
    println!("Command: {}", command);
    println!("Duration: {:.4} seconds", duration.as_secs_f64());
    println!("Success: {}", success);
    println!("Output: \n{}", stdout);
    println!("--------------------------------------------------");
}

#[test]
fn test_command_performance() {
    // Setup test environment
    setup();

    // Warmup run (not measured)
    measure_command(&["init", "-y"]);
    cleanup();
    setup();

    // Define test cases
    let test_cases = [
        ("init -y", &["init", "-y"] as &[&str]),
        ("list (empty)", &["list"]),
        ("install lodash", &["install", "lodash"]),
        ("list (after install)", &["list"]),
        ("install multiple packages", &["install", "react", "react-dom", "express"]),
        ("install with --save-dev", &["install", "-D", "jest"]),
        ("list --dev", &["list", "--dev"]),
        ("list --production", &["list", "--production"]),
    ];

    // Store results
    let mut results = Vec::new();

    // Run tests
    for (name, cmd) in &test_cases {
        let mut durations = Vec::with_capacity(ITERATIONS);
        let mut stdout = String::new();
        let mut success = false;

        for i in 0..ITERATIONS {
            // Only show output for the first iteration to avoid cluttering the terminal
            let (duration, this_stdout, this_success) = measure_command(cmd);
            durations.push(duration);
            
            if i == 0 {
                stdout = this_stdout;
                success = this_success;
            }
        }

        // Calculate average
        let avg_duration = durations.iter().sum::<Duration>() / ITERATIONS as u32;
        
        // Output result
        print_result(name, avg_duration, success, &stdout);
        
        // Store result
        results.push((name, avg_duration));
    }

    // Print summary of performance
    println!("\n=== Performance Summary (Averaged over {} runs) ===", ITERATIONS);
    let mut total_time = Duration::new(0, 0);
    
    for (name, duration) in &results {
        println!("{}: {:.4} seconds", name, duration.as_secs_f64());
        total_time += *duration;
    }
    
    println!("Average total test time: {:.4} seconds", total_time.as_secs_f64());

    // Cleanup test environment
    cleanup();
}
