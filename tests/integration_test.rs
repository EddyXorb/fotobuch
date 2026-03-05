//! Integration tests for photobook-solver
//!
//! These tests validate the end-to-end behavior of the solver.

use photobook_solver::{Canvas, GaConfig, SolverRequest, load_photos_from_dir, run_solver};
use std::fs;
use std::path::PathBuf;

const TEST_PHOTOS_DIR: &str = "tests/fixtures/test_photos";

/// Helper to get the test photos directory path
fn test_photos_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TEST_PHOTOS_DIR)
}

#[test]
fn test_load_test_photos() {
    let photo_dir = test_photos_path();
    let photos = load_photos_from_dir(&photo_dir).expect("Failed to load test photos");

    assert_eq!(photos.len(), 3, "Expected 3 test photos in fixtures");

    // Verify photos have valid properties
    for photo_info in &photos {
        assert!(
            photo_info.photo.aspect_ratio > 0.0,
            "Photo should have positive aspect ratio"
        );
        assert!(
            !photo_info.photo.group.is_empty(),
            "Photo should have a group"
        );
        assert!(photo_info.path.exists(), "Photo file should exist");
    }
}

#[test]
fn test_end_to_end_solver() {
    let photo_dir = test_photos_path();
    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_output.typ");

    let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
    let ga_config = GaConfig {
        seed: 1772727622,
        ..GaConfig::default()
    };

    let request = SolverRequest::new(photo_dir.clone(), output_path.clone(), canvas, ga_config);

    // Run the solver and get the BookLayout
    let book_layout = run_solver(&request).expect("Solver should complete successfully");

    // Verify BookLayout structure
    assert!(!book_layout.is_empty(), "Book layout should not be empty");
    assert_eq!(book_layout.page_count(), 1, "Should have exactly 1 page");
    assert_eq!(
        book_layout.total_photo_count(),
        3,
        "Should place all 3 test photos"
    );

    // Verify first page properties
    let first_page = &book_layout.pages[0];
    assert_eq!(
        first_page.placements.len(),
        3,
        "First page should have 3 photos"
    );
    assert_eq!(first_page.canvas.width, 297.0, "Canvas width should match");
    assert_eq!(
        first_page.canvas.height, 210.0,
        "Canvas height should match"
    );

    // Verify all placements are valid
    for placement in &first_page.placements {
        // Check photo index is valid
        assert!(
            placement.photo_idx < 3,
            "Photo index {} should be < 3",
            placement.photo_idx
        );

        // Check dimensions are positive
        assert!(placement.w > 0.0, "Photo width should be positive");
        assert!(placement.h > 0.0, "Photo height should be positive");

        // Check placement is within canvas bounds
        assert!(
            placement.x >= 0.0,
            "Photo x position should be non-negative"
        );
        assert!(
            placement.y >= 0.0,
            "Photo y position should be non-negative"
        );
        assert!(
            placement.right() <= canvas.width,
            "Photo should not exceed canvas width"
        );
        assert!(
            placement.bottom() <= canvas.height,
            "Photo should not exceed canvas height"
        );
    }

    // Check coverage ratio is reasonable (should use some of the canvas)
    let coverage = first_page.coverage_ratio();
    assert!(
        coverage > 0.1 && coverage <= 1.0,
        "Coverage ratio {} should be reasonable",
        coverage
    );

    // Verify output file was created
    assert!(output_path.exists(), "Output file should be created");

    // Read and validate output
    let content = fs::read_to_string(&output_path).expect("Should read output file");
    assert!(
        content.contains("#set page"),
        "Output should contain page setup"
    );
    assert!(
        content.contains("297mm"),
        "Output should contain canvas width"
    );
    assert!(
        content.contains("210mm"),
        "Output should contain canvas height"
    );
    assert!(
        content.matches("#place").count() >= 1,
        "Output should contain photo placements"
    );

    // Cleanup
    let _ = fs::remove_file(output_path);
}

#[test]
fn test_baseline_snapshot() {
    // This test captures the exact output to detect unintended changes during refactoring
    let photo_dir = test_photos_path();

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_baseline.typ");

    let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
    let ga_config = GaConfig {
        seed: 1772727622,
        ..GaConfig::default()
    };

    let request = SolverRequest::new(photo_dir, output_path.clone(), canvas, ga_config);

    let book_layout = run_solver(&request).expect("Solver should succeed");

    // Verify BookLayout consistency
    assert_eq!(book_layout.page_count(), 1, "Should have 1 page");
    assert_eq!(book_layout.total_photo_count(), 3, "Should have 3 photos");

    let content = fs::read_to_string(&output_path).expect("Should read output");

    // Snapshot assertions - these should remain stable after refactoring
    assert!(content.contains("#set page(width: 297mm, height: 210mm, margin: 0pt)"));

    // Should contain exactly 3 images (one per photo)
    let image_count = content.matches("image(").count();
    assert_eq!(
        image_count, 3,
        "Should have exactly 3 image placements for 3 photos"
    );

    // All test photos should be referenced
    assert!(
        content.contains("test.jpg") || content.contains("group1"),
        "Should reference test.jpg"
    );
    assert!(
        content.contains("test2.jpg") || content.contains("group2"),
        "Should reference test2.jpg"
    );
    assert!(
        content.contains("test3.jpg") || content.contains("group2"),
        "Should reference test3.jpg"
    );

    // Should have proper Typst structure
    let place_count = content.matches("#place(top + left").count();
    assert_eq!(place_count, 3, "Should have 3 placement directives");

    // Cleanup
    let _ = fs::remove_file(output_path);
}

#[test]
fn test_deterministic_output() {
    // Run solver twice with same seed, verify identical output
    let photo_dir = test_photos_path();

    let output1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_run1.typ");
    let output2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_run2.typ");

    let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
    let ga_config = GaConfig {
        seed: 42,
        ..GaConfig::default()
    };

    // First run
    let request1 = SolverRequest::new(
        photo_dir.clone(),
        output1.clone(),
        canvas.clone(),
        ga_config.clone(),
    );
    let book_layout1 = run_solver(&request1).expect("First run should succeed");

    // Second run
    let request2 = SolverRequest::new(photo_dir, output2.clone(), canvas, ga_config);
    let book_layout2 = run_solver(&request2).expect("Second run should succeed");

    // Compare BookLayout properties
    assert_eq!(
        book_layout1.page_count(),
        book_layout2.page_count(),
        "Same seed should produce same page count"
    );
    assert_eq!(
        book_layout1.total_photo_count(),
        book_layout2.total_photo_count(),
        "Same seed should produce same photo count"
    );

    // Compare first page placements
    let page1 = &book_layout1.pages[0];
    let page2 = &book_layout2.pages[0];
    assert_eq!(
        page1.placements.len(),
        page2.placements.len(),
        "Same seed should produce same number of placements"
    );

    for (p1, p2) in page1.placements.iter().zip(page2.placements.iter()) {
        assert_eq!(p1.photo_idx, p2.photo_idx, "Photo indices should match");
        assert!((p1.x - p2.x).abs() < 1e-6, "Photo x positions should match");
        assert!((p1.y - p2.y).abs() < 1e-6, "Photo y positions should match");
        assert!((p1.w - p2.w).abs() < 1e-6, "Photo widths should match");
        assert!((p1.h - p2.h).abs() < 1e-6, "Photo heights should match");
    }

    // Compare file outputs
    let content1 = fs::read_to_string(&output1).expect("Should read first output");
    let content2 = fs::read_to_string(&output2).expect("Should read second output");

    assert_eq!(
        content1, content2,
        "Same seed should produce identical output"
    );

    // Cleanup
    let _ = fs::remove_file(output1);
    let _ = fs::remove_file(output2);
}

#[test]
fn test_different_configurations() {
    let photo_dir = test_photos_path();

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_config.typ");

    // Test with A4 landscape
    let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
    let ga_config = GaConfig {
        seed: 123,
        ..GaConfig::default()
    };

    let request = SolverRequest::new(photo_dir, output_path.clone(), canvas, ga_config);

    let book_layout = run_solver(&request).expect("Solver should work with standard config");

    // Verify BookLayout
    assert!(!book_layout.is_empty(), "Book layout should not be empty");
    assert_eq!(book_layout.page_count(), 1, "Should have 1 page");
    assert_eq!(
        book_layout.total_photo_count(),
        3,
        "Should place all 3 photos"
    );

    // Verify canvas dimensions are preserved
    let page = &book_layout.pages[0];
    assert_eq!(page.canvas.width, 297.0, "Canvas width should be 297mm");
    assert_eq!(page.canvas.height, 210.0, "Canvas height should be 210mm");

    let content = fs::read_to_string(&output_path).expect("Should read output");
    assert!(content.contains("297mm"), "Should have correct width");
    assert!(content.contains("210mm"), "Should have correct height");

    // Cleanup
    let _ = fs::remove_file(output_path);
}

#[test]
fn test_cli_exact_output() {
    use std::process::Command;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_path = manifest_dir.join("target").join("test_cli_output.typ");

    // Clean up any previous output
    let _ = fs::remove_file(&output_path);

    // Run the CLI using the compiled binary
    let binary_path = manifest_dir
        .join("target")
        .join("debug")
        .join("photobook-solver");

    let output = Command::new(&binary_path)
        .args([
            "-i",
            "tests/fixtures/test_photos/",
            "-o",
            "target/test_cli_output.typ",
            "--seed",
            "1772727622", // Use fixed seed for deterministic output
        ])
        .current_dir(&manifest_dir)
        .output()
        .expect("Failed to execute CLI");

    assert!(
        output.status.success(),
        "CLI should exit successfully. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the generated output
    let content = fs::read_to_string(&output_path).expect("Should read CLI output");

    // Verify the exact format of the generated Typst file
    assert!(
        content.starts_with("// Generated by photobook-solver\n"),
        "File should start with comment"
    );

    assert!(
        content.contains("#set page(width: 297mm, height: 210mm, margin: 0pt)"),
        "File should contain correct page setup"
    );

    // Verify each placement line has the expected format
    assert!(
        content.contains("#place(top + left, dx:"),
        "Should contain place directives"
    );
    assert!(
        content.contains("block(width:"),
        "Should contain block directives"
    );
    assert!(
        content.contains("image(\"tests/fixtures/test_photos/"),
        "Should contain correct image paths"
    );

    // Count the number of placements (should be 3 for the 3 test photos)
    let placement_count = content.matches("#place(").count();
    assert_eq!(placement_count, 3, "Should have exactly 3 photo placements");

    // Verify exact positions and dimensions for deterministic output with seed 1772727622
    // These specific values ensure the solver produces consistent results

    // Photo 1: test3.jpg (group2) - large photo on the right
    assert!(
        content.contains("#place(top + left, dx: 100.67mm, dy: 6.83mm, block(width: 196.33mm, height: 196.33mm, clip: true, image(\"tests/fixtures/test_photos/group2/test3.jpg\""),
        "test3.jpg should be at x=100.67mm, y=6.83mm with size 196.33x196.33mm"
    );

    // Photo 2: test.jpg (group1) - small photo on top left
    assert!(
        content.contains("#place(top + left, dx: 0.00mm, dy: 6.83mm, block(width: 95.67mm, height: 95.67mm, clip: true, image(\"tests/fixtures/test_photos/group1/test.jpg\""),
        "test.jpg should be at x=0.00mm, y=6.83mm with size 95.67x95.67mm"
    );

    // Photo 3: test2.jpg (group2) - small photo on bottom left
    assert!(
        content.contains("#place(top + left, dx: 0.00mm, dy: 107.50mm, block(width: 95.67mm, height: 95.67mm, clip: true, image(\"tests/fixtures/test_photos/group2/test2.jpg\""),
        "test2.jpg should be at x=0.00mm, y=107.50mm with size 95.67x95.67mm"
    );

    // Verify the complete format of one placement for completeness
    assert!(
        content.contains("fit: \"cover\")))"),
        "Should have proper closing with fit: cover"
    );

    // Cleanup
    let _ = fs::remove_file(output_path);
}
