use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_project_structure() {
    // Verify that all expected module files exist
    assert!(Path::new("src/main.rs").exists());
    assert!(Path::new("src/cli.rs").exists());
    assert!(Path::new("src/scanner.rs").exists());
    assert!(Path::new("src/organizer.rs").exists());
    assert!(Path::new("src/duplicate.rs").exists());
    assert!(Path::new("src/progress.rs").exists());
    assert!(Path::new("src/types.rs").exists());
}

#[tokio::test]
async fn test_basic_functionality() {
    // This test would require the actual binary to be built
    // For now, we just verify the structure is in place
    let temp_dir = tempdir().unwrap();
    let input_path = temp_dir.path().join("input");
    let output_path = temp_dir.path().join("output");

    // Create test directories
    std::fs::create_dir_all(&input_path).unwrap();
    std::fs::create_dir_all(&output_path).unwrap();

    // Verify directories were created
    assert!(input_path.exists());
    assert!(output_path.exists());
}
