use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn measure_command(cmd: &[&str]) -> (Duration, String, bool) {
    let start = Instant::now();

    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(cmd)
        .output()
        .expect("Failed to execute command");

    let elapsed = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let success = output.status.success();

    (elapsed, stdout, success)
}

fn setup() {
    // Create a test directory
    let test_dir = Path::new("test_project");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).expect("Failed to remove test directory");
    }
    fs::create_dir(test_dir).expect("Failed to create test directory");

    // Change to the test directory
    std::env::set_current_dir(test_dir).expect("Failed to change directory");
}

fn cleanup() {
    // Change back to the parent directory
    std::env::set_current_dir("..").expect("Failed to change directory");

    // Remove the test directory
    fs::remove_dir_all("test_project").expect("Failed to remove test directory");
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

    // Test init command
    let (init_duration, init_stdout, init_success) = measure_command(&["init", "-y"]);
    print_result("init -y", init_duration, init_success, &init_stdout);

    // Test list command when empty
    let (list_empty_duration, list_empty_stdout, list_empty_success) = measure_command(&["list"]);
    print_result(
        "list (empty)",
        list_empty_duration,
        list_empty_success,
        &list_empty_stdout,
    );

    // Test install command for one package
    let (install_single_duration, install_single_stdout, install_single_success) =
        measure_command(&["install", "lodash"]);
    print_result(
        "install lodash",
        install_single_duration,
        install_single_success,
        &install_single_stdout,
    );

    // Test list command after installation
    let (list_after_duration, list_after_stdout, list_after_success) = measure_command(&["list"]);
    print_result(
        "list (after install)",
        list_after_duration,
        list_after_success,
        &list_after_stdout,
    );

    // Test install command for multiple packages
    let (install_multi_duration, install_multi_stdout, install_multi_success) =
        measure_command(&["install", "react", "react-dom", "express"]);
    print_result(
        "install multiple packages",
        install_multi_duration,
        install_multi_success,
        &install_multi_stdout,
    );

    // Test install command with --save-dev flag
    let (install_dev_duration, install_dev_stdout, install_dev_success) =
        measure_command(&["install", "-D", "jest"]);
    print_result(
        "install with --save-dev",
        install_dev_duration,
        install_dev_success,
        &install_dev_stdout,
    );

    // Test list with filters
    let (list_dev_duration, list_dev_stdout, list_dev_success) =
        measure_command(&["list", "--dev"]);
    print_result(
        "list --dev",
        list_dev_duration,
        list_dev_success,
        &list_dev_stdout,
    );

    let (list_prod_duration, list_prod_stdout, list_prod_success) =
        measure_command(&["list", "--production"]);
    print_result(
        "list --production",
        list_prod_duration,
        list_prod_success,
        &list_prod_stdout,
    );

    // Print summary of performance
    println!("\n=== Performance Summary ===");
    println!("init -y: {:.4} seconds", init_duration.as_secs_f64());
    println!(
        "list (empty): {:.4} seconds",
        list_empty_duration.as_secs_f64()
    );
    println!(
        "install lodash: {:.4} seconds",
        install_single_duration.as_secs_f64()
    );
    println!(
        "list (after install): {:.4} seconds",
        list_after_duration.as_secs_f64()
    );
    println!(
        "install multiple: {:.4} seconds",
        install_multi_duration.as_secs_f64()
    );
    println!(
        "install --save-dev: {:.4} seconds",
        install_dev_duration.as_secs_f64()
    );
    println!("list --dev: {:.4} seconds", list_dev_duration.as_secs_f64());
    println!(
        "list --production: {:.4} seconds",
        list_prod_duration.as_secs_f64()
    );

    // Get total test time
    let total_time = init_duration
        + list_empty_duration
        + install_single_duration
        + list_after_duration
        + install_multi_duration
        + install_dev_duration
        + list_dev_duration
        + list_prod_duration;
    println!("Total test time: {:.4} seconds", total_time.as_secs_f64());

    // Cleanup test environment
    cleanup();
}
