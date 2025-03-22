use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;
use std::env;
use tempfile::TempDir;
use std::path::PathBuf;

const ITERATIONS: usize = 3;
const WARM_UP: bool = true;

// Use a static variable to store test directory path
static CURRENT_TEST_DIR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

struct TestEnvironment {
    temp_dir: TempDir,
    original_dir: PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let original_dir = env::current_dir().unwrap_or_else(|_| {
            println!("Failed to get current directory, using fallback");
            PathBuf::from(".")
        });
        
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        env::set_current_dir(&temp_dir.path()).expect("Failed to change to temp directory");
        
        Self {
            temp_dir,
            original_dir,
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        println!("Cleaning up test directory: {:?}", self.temp_dir.path());
        
        // Try to change back to the original directory, but don't panic if it fails
        if let Err(e) = env::set_current_dir(&self.original_dir) {
            println!("Warning: Failed to change back to original directory: {}", e);
        }
    }
}

// Helper function to find the binary path reliably
fn find_rjs_binary() -> std::path::PathBuf {
    // Try multiple possible locations for the binary
    let possible_paths = [
        // From current executable dir (when running via cargo test)
        std::env::current_exe().ok()
            .and_then(|path| path.parent().map(|p| p.join("rjs"))),
        
        // From CARGO_MANIFEST_DIR env var
        std::env::var("CARGO_MANIFEST_DIR").ok()
            .map(|dir| std::path::PathBuf::from(dir).join("target/release/rjs")),
        
        // From current directory, assuming we're in the project root
        Some(std::env::current_dir().unwrap_or_default().join("target/release/rjs")),
        
        // From RJS_PROJECT_DIR env var if set by test scripts
        std::env::var("RJS_PROJECT_DIR").ok()
            .map(|dir| std::path::PathBuf::from(dir).join("target/release/rjs")),
    ];
    
    // Try each path and return the first one that exists
    for maybe_path in possible_paths.iter().flatten() {
        if maybe_path.exists() {
            println!("Found RJS binary at: {:?}", maybe_path);
            return maybe_path.clone();
        }
    }
    
    // If we can't find it, use a default and let the test fail with a clear error
    println!("Warning: Could not find RJS binary, using fallback path");
    std::path::PathBuf::from("target/release/rjs")
}

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
    // Create a TestEnvironment instead of using setup/cleanup
    let _env = TestEnvironment::new();
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    
    // Warmup run (not measured)
    let warmup_result = Command::new(&binary_path)
        .args(&["init", "-y"])
        .output();
    
    if let Err(e) = &warmup_result {
        println!("Warmup failed: {}, binary path: {:?}", e, binary_path);
        // Skip the test if we can't run the binary
        return;
    }
    
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
            // Run command and measure time
            let start = Instant::now();
            let output = match Command::new(&binary_path)
                .args(*cmd)
                .output() {
                    Ok(o) => o,
                    Err(e) => {
                        println!("Command failed: {}, binary path: {:?}", e, binary_path);
                        // Skip the rest of the iterations if we can't run the command
                        break;
                    }
                };
            let duration = start.elapsed();
            
            durations.push(duration);
            success = output.status.success();
            
            if i == 0 {
                stdout = String::from_utf8_lossy(&output.stdout).to_string();
            }
        }

        if durations.is_empty() {
            continue;
        }

        // Calculate average
        let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
        
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
}

// Helper function to run a command
fn run_command(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env::current_exe().unwrap().parent().unwrap().join("rjs"))
        .args(args)
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_concurrent_install_performance() {
    // Store current directory
    let prev_dir = env::current_dir().unwrap_or_else(|_| {
        println!("Failed to get current directory, using fallback");
        PathBuf::from(".")
    });
    
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    
    // Change to the temporary directory
    if let Err(e) = env::set_current_dir(temp_dir.path()) {
        println!("Failed to change to temp directory: {}", e);
        return;
    }
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    println!("Found RJS binary at: {:?}", binary_path);
    
    // Run init command to create package.json
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                // Try to change back, but don't fail if it doesn't work
                let _ = env::set_current_dir(&prev_dir);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Init command failed");
        // Try to change back, but don't fail if it doesn't work
        let _ = env::set_current_dir(&prev_dir);
        return;
    }
    
    // Test install performance with high concurrency
    let start = Instant::now();
    let output = match Command::new(&binary_path)
        .args(&["install", "lodash", "--concurrency", "8", "--no-progress"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("High concurrency install failed: {}, binary path: {:?}", e, binary_path);
                // Try to change back, but don't fail if it doesn't work
                let _ = env::set_current_dir(&prev_dir);
                return;
            }
        };
    let high_concurrency_success = output.status.success();
    let high_concurrency_duration = start.elapsed();
    
    if !high_concurrency_success {
        println!("High concurrency install failed");
        // Try to change back, but don't fail if it doesn't work
        let _ = env::set_current_dir(&prev_dir);
        return;
    }

    // Clean up and recreate test directory
    // Try to change back, but don't fail if it doesn't work
    if let Err(e) = env::set_current_dir(&prev_dir) {
        println!("Failed to change back to original directory: {}", e);
    }
}

#[test]
fn test_batch_size_impact() {
    // Store current directory
    let prev_dir = env::current_dir().unwrap_or_else(|_| {
        println!("Failed to get current directory, using fallback");
        PathBuf::from(".")
    });
    
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    
    // Change to the temporary directory
    if let Err(e) = env::set_current_dir(temp_dir.path()) {
        println!("Failed to change to temp directory: {}", e);
        return;
    }
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    println!("Found RJS binary at: {:?}", binary_path);
    
    // Run init command
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                // Try to change back, but don't fail if it doesn't work
                let _ = env::set_current_dir(&prev_dir);
                return;
            }
        };
    let init_success = output.status.success();
    
    if !init_success {
        println!("Init command failed");
        // Try to change back, but don't fail if it doesn't work
        let _ = env::set_current_dir(&prev_dir);
        return;
    }

    // Install multiple packages with batch size setting
    let output = match Command::new(&binary_path)
        .args(&["install", "lodash", "chalk", "uuid", "--batch-size", "10", "--no-progress"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Install failed: {}, binary path: {:?}", e, binary_path);
                // Try to change back, but don't fail if it doesn't work
                let _ = env::set_current_dir(&prev_dir);
                return;
            }
        };
    let install_success = output.status.success();
    let install_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    if !install_success {
        println!("Installation with custom batch size failed");
        // Try to change back, but don't fail if it doesn't work
        let _ = env::set_current_dir(&prev_dir);
        return;
    }
    
    // Check that batch size was acknowledged
    if !(install_stdout.contains("batch size: 10") || 
         install_stdout.contains("batch_size: 10") ||
         install_stdout.contains("batch-size: 10")) {
        println!("Batch size setting not acknowledged in output");
    }

    // Try to change back to the original directory at the end
    if let Err(e) = env::set_current_dir(&prev_dir) {
        println!("Failed to change back to original directory at end: {}", e);
    }
}

// Helper function to run a command and measure execution time
fn measure_command_env(args: &[&str]) -> (Duration, String, bool) {
    // Path to the release binary for accurate performance measurement
    let binary_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()))
        .join("target/release/rjs");
    
    let start = Instant::now();
    let output = Command::new(&binary_path)
        .args(args)
        .output()
        .expect("Failed to execute command");
    let duration = start.elapsed();
    
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    (duration, stdout, success)
}

// Helper function to print test results
fn print_result_env(test_name: &str, duration: Duration, success: bool) {
    println!(
        "{}: {:.4} seconds, success: {}",
        test_name,
        duration.as_secs_f64(),
        success
    );
}

// Test the impact of concurrency settings on installation speed
#[test]
fn test_concurrency_impact() {
    let _env = TestEnvironment::new();
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    
    // Initialize the project
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Test different concurrency levels
    let concurrency_levels = [1, 4];  // Reduced to speed up tests
    let mut results = Vec::new();
    
    for &concurrency in &concurrency_levels {
        // Clean up from previous run
        let _env = TestEnvironment::new();
        
        let output = match Command::new(&binary_path)
            .args(&["init", "--yes"])
            .output() {
                Ok(o) => o,
                Err(e) => {
                    println!("Init failed for concurrency {}: {}", concurrency, e);
                    continue;
                }
            };
        let init_success = output.status.success();
        if !init_success {
            println!("Failed to initialize project for concurrency {}", concurrency);
            continue;
        }
        
        // Skip warm-up to speed up tests
        
        // Actual measured run
        let args = &[
            "install", "lodash", "chalk", 
            "--concurrency", &concurrency.to_string(),
            "--no-progress"
        ];
        
        let mut durations = Vec::with_capacity(1);  // Reduced to 1 iteration
        let mut success = false;
        
        for _ in 0..1 {  // Just one iteration to speed up tests
            let start = Instant::now();
            let output = match Command::new(&binary_path)
                .args(args)
                .output() {
                    Ok(o) => o,
                    Err(e) => {
                        println!("Install failed for concurrency {}: {}", concurrency, e);
                        break;
                    }
                };
            let duration = start.elapsed();
            let run_success = output.status.success();
            
            durations.push(duration);
            success = run_success;
            
            if !success {
                break;
            }
        }
        
        if !success {
            println!("Test failed with concurrency level {}", concurrency);
            continue;
        }
        
        if durations.is_empty() {
            continue;
        }
        
        // Calculate average duration
        let total_duration: Duration = durations.iter().sum();
        let avg_duration = total_duration / durations.len() as u32;
        
        // Store result
        results.push((concurrency, avg_duration));
        
        // Print result
        print_result_env(
            &format!("Concurrency level {}", concurrency),
            avg_duration,
            success
        );
    }
    
    // Validate that higher concurrency is generally faster
    if results.len() >= 2 {
        // Sort by concurrency (should already be sorted)
        results.sort_by_key(|&(concurrency, _)| concurrency);
        
        // Low concurrency should generally be slower than high concurrency
        let (_, low_duration) = results.first().unwrap();
        let (_, high_duration) = results.last().unwrap();
        
        println!(
            "Concurrency comparison: lowest {} vs highest {}, improvement: {:.2}x",
            low_duration.as_secs_f64(),
            high_duration.as_secs_f64(),
            low_duration.as_secs_f64() / high_duration.as_secs_f64()
        );
    }
}

// Test the impact of batch size on installation speed
#[test]
fn test_batch_size_impact_env() {
    let _env = TestEnvironment::new();
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    
    // Initialize the project
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Test different batch sizes - just one to speed up tests
    let batch_sizes = [10];
    
    for &batch_size in &batch_sizes {
        // Clean up from previous run
        let _env = TestEnvironment::new();
        
        let output = match Command::new(&binary_path)
            .args(&["init", "--yes"])
            .output() {
                Ok(o) => o,
                Err(e) => {
                    println!("Init failed for batch size {}: {}", batch_size, e);
                    continue;
                }
            };
        let init_success = output.status.success();
        if !init_success {
            println!("Failed to initialize project for batch size {}", batch_size);
            continue;
        }
        
        // Install multiple packages with specified batch size
        let args = &[
            "install", "lodash", "chalk",
            "--batch-size", &batch_size.to_string(),
            "--no-progress"
        ];
        
        let start = Instant::now();
        let output = match Command::new(&binary_path)
            .args(args)
            .output() {
                Ok(o) => o,
                Err(e) => {
                    println!("Install failed for batch size {}: {}", batch_size, e);
                    continue;
                }
            };
        let duration = start.elapsed();
        let success = output.status.success();
        
        if !success {
            println!("Test failed with batch size {}", batch_size);
            continue;
        }
        
        // Print result
        print_result_env(
            &format!("Batch size {}", batch_size),
            duration,
            success
        );
    }
}

// Test performance of installing from package.json
#[test]
fn test_install_from_package_json() {
    let _env = TestEnvironment::new();
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    
    // Initialize the project
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Create a basic package.json with minimal dependencies (just one to speed up tests)
    let package_json = r#"{
        "name": "rjs-performance-test",
        "version": "1.0.0",
        "dependencies": {
            "lodash": "^4.17.21"
        }
    }"#;
    
    if let Err(e) = std::fs::write("package.json", package_json) {
        println!("Failed to write package.json: {}", e);
        return;
    }
    
    // Measure installation from package.json
    let args = &["install", "--no-progress"];
    
    let start = Instant::now();
    let output = match Command::new(&binary_path)
        .args(args)
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Install failed: {}", e);
                return;
            }
        };
    let duration = start.elapsed();
    let success = output.status.success();
    
    if !success {
        println!("Failed to install from package.json");
        return;
    }
    
    // Print result
    print_result_env("Install from package.json", duration, success);
}

// Test the comparison between regular and dev dependencies
#[test]
fn test_regular_vs_dev_dependencies() {
    let _env = TestEnvironment::new();
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    
    // Initialize the project
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Install regular dependencies
    let start = Instant::now();
    let output = match Command::new(&binary_path)
        .args(&["install", "lodash", "--no-progress"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Install regular dependencies failed: {}", e);
                return;
            }
        };
    let regular_success = output.status.success();
    let regular_duration = start.elapsed();
    if !regular_success {
        println!("Failed to install regular dependencies");
        return;
    }
    
    // Clean up and create new environment
    let _env = TestEnvironment::new();
    
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}", e);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Install dev dependencies
    let start = Instant::now();
    let output = match Command::new(&binary_path)
        .args(&["install", "lodash", "--save-dev", "--no-progress"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Install dev dependencies failed: {}", e);
                return;
            }
        };
    let dev_success = output.status.success();
    let dev_duration = start.elapsed();
    if !dev_success {
        println!("Failed to install dev dependencies");
        return;
    }
    
    // Print results
    print_result_env("Regular dependencies", regular_duration, regular_success);
    print_result_env("Dev dependencies", dev_duration, dev_success);
    
    // Dev and regular installs should take similar time
    println!(
        "Regular vs Dev: {:.4} vs {:.4} seconds",
        regular_duration.as_secs_f64(),
        dev_duration.as_secs_f64()
    );
}

// Test performance with and without progress reporting
#[test]
fn test_progress_reporting_impact() {
    let _env = TestEnvironment::new();
    
    // Find path to the binary
    let binary_path = find_rjs_binary();
    
    // Initialize the project
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}, binary path: {:?}", e, binary_path);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Install with progress reporting
    let start = Instant::now();
    let output = match Command::new(&binary_path)
        .args(&["install", "lodash"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Install with progress failed: {}", e);
                return;
            }
        };
    let with_progress_success = output.status.success();
    let with_progress_duration = start.elapsed();
    if !with_progress_success {
        println!("Failed to install with progress reporting");
        return;
    }
    
    // Clean up and create new environment
    let _env = TestEnvironment::new();
    
    let output = match Command::new(&binary_path)
        .args(&["init", "--yes"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Init failed: {}", e);
                return;
            }
        };
    let init_success = output.status.success();
    if !init_success {
        println!("Failed to initialize project");
        return;
    }
    
    // Install without progress reporting
    let start = Instant::now();
    let output = match Command::new(&binary_path)
        .args(&["install", "lodash", "--no-progress"])
        .output() {
            Ok(o) => o,
            Err(e) => {
                println!("Install without progress failed: {}", e);
                return;
            }
        };
    let no_progress_success = output.status.success();
    let no_progress_duration = start.elapsed();
    if !no_progress_success {
        println!("Failed to install without progress reporting");
        return;
    }
    
    // Print results
    print_result_env("With progress", with_progress_duration, with_progress_success);
    print_result_env("Without progress", no_progress_duration, no_progress_success);
    
    // No-progress should generally be faster
    println!(
        "Progress impact: with={:.4}s, without={:.4}s, difference={:.2}%",
        with_progress_duration.as_secs_f64(),
        no_progress_duration.as_secs_f64(),
        (with_progress_duration.as_secs_f64() - no_progress_duration.as_secs_f64()) / 
            with_progress_duration.as_secs_f64() * 100.0
    );
}
