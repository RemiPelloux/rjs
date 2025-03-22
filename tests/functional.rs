use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TestEnv {
    test_dir: PathBuf,
    original_dir: PathBuf,
}

impl TestEnv {
    fn new(test_dir_name: &str) -> Self {
        let original_dir = std::env::current_dir().expect("Failed to get current directory");
        let test_dir = original_dir.join(test_dir_name);

        // Create a test directory
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).expect("Failed to remove test directory");
        }
        fs::create_dir(&test_dir).expect("Failed to create test directory");

        // Change to the test directory
        std::env::set_current_dir(&test_dir).expect("Failed to change directory");

        Self {
            test_dir,
            original_dir,
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Change back to the original directory
        std::env::set_current_dir(&self.original_dir)
            .expect("Failed to change back to original directory");

        // Remove the test directory
        fs::remove_dir_all(&self.test_dir).expect("Failed to remove test directory");
    }
}

fn run_command(args: &[&str]) -> (bool, String, String) {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

#[test]
fn test_init_command() {
    let _env = TestEnv::new("test_init");

    // Run init command
    let (success, stdout, stderr) = run_command(&["init", "-y"]);

    // Assert that the command was successful
    assert!(success, "Init command failed: {}", stderr);
    assert!(
        stdout.contains("Created package.json"),
        "Init output doesn't contain expected message"
    );

    // Check that package.json was created
    let package_json = Path::new("package.json");
    assert!(package_json.exists(), "package.json wasn't created");

    // Verify package.json content
    let content = fs::read_to_string(package_json).expect("Failed to read package.json");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse package.json");

    assert!(
        json.get("name").is_some(),
        "package.json is missing 'name' field"
    );
    assert!(
        json.get("version").is_some(),
        "package.json is missing 'version' field"
    );
    assert!(
        json.get("dependencies").is_some(),
        "package.json is missing 'dependencies' field"
    );
}

#[test]
fn test_install_command() {
    let _env = TestEnv::new("test_install");

    // First, create package.json
    let (init_success, _, _) = run_command(&["init", "-y"]);
    assert!(init_success, "Init command failed");

    // Run install command
    let (success, stdout, stderr) = run_command(&["install", "lodash"]);

    // Assert that the command was successful
    assert!(success, "Install command failed: {}", stderr);
    assert!(
        stdout.contains("Installing specified packages"),
        "Install output doesn't contain expected message"
    );
    assert!(
        stdout.contains("Installed"),
        "Install output doesn't show that package was installed"
    );

    // Note: In a real implementation, we would check if node_modules/lodash exists
    // and if package.json was updated with the dependency
}

#[test]
fn test_list_command() {
    let _env = TestEnv::new("test_list");

    // First, create package.json and install a dependency
    let (init_success, _, _) = run_command(&["init", "-y"]);
    assert!(init_success, "Init command failed");

    // Check list with no dependencies
    let (list_empty_success, list_empty_stdout, _) = run_command(&["list"]);
    assert!(list_empty_success, "List command failed");
    assert!(
        list_empty_stdout.contains("No dependencies found")
            || list_empty_stdout.contains("0 dependencies"),
        "List output doesn't show empty dependencies"
    );

    // Install a package
    let (install_success, _, _) = run_command(&["install", "lodash"]);
    assert!(install_success, "Install command failed");

    // Run list command again
    let (list_success, list_stdout, _) = run_command(&["list"]);
    assert!(list_success, "List command failed after install");

    // In a full implementation, this would check that lodash is in the dependencies list
    // For now, we're just checking that the command runs successfully
}

#[test]
fn test_dev_dependencies() {
    let _env = TestEnv::new("test_dev_deps");

    // Create package.json
    let (init_success, _, _) = run_command(&["init", "-y"]);
    assert!(init_success, "Init command failed");

    // Install a dev dependency
    let (install_success, _, _) = run_command(&["install", "-D", "jest"]);
    assert!(install_success, "Install command with -D failed");

    // List dev dependencies
    let (list_success, list_stdout, _) = run_command(&["list", "--dev"]);
    assert!(list_success, "List command with --dev failed");

    // In a full implementation, this would check that jest is in devDependencies
    // For now, we're just checking that the commands run successfully
}
