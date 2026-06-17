use nahpu_archive::archive::{ZipArchive, ZipExtractor};
use std::fs;

#[test]
fn test_archive_and_extract() {
    // Setup temporary directories
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_dir = temp_dir.path().join("source");
    let output_zip = temp_dir.path().join("archive.zip");
    let extract_dir = temp_dir.path().join("extracted");

    fs::create_dir_all(&base_dir).unwrap();
    fs::create_dir_all(&extract_dir).unwrap();

    // Create some dummy files to archive
    let file1_path = base_dir.join("file1.txt");
    let file2_dir = base_dir.join("sub_dir");
    fs::create_dir_all(&file2_dir).unwrap();
    let file2_path = file2_dir.join("file2.txt");

    fs::write(&file1_path, "Hello, file 1!").unwrap();
    fs::write(&file2_path, "Hello, file 2!").unwrap();

    let files_to_archive = vec![file1_path, file2_path];

    // Archive
    let zip_archive = ZipArchive::new(&base_dir, None, &output_zip, &files_to_archive);
    zip_archive.write().expect("Failed to write zip archive");

    assert!(output_zip.exists(), "Zip archive was not created");

    // Extract
    let zip_extractor = ZipExtractor::new(&output_zip, &extract_dir);
    zip_extractor
        .extract()
        .expect("Failed to extract zip archive");

    // Verify
    let extracted_file1 = extract_dir.join("file1.txt");
    let extracted_file2 = extract_dir.join("sub_dir").join("file2.txt");

    assert!(extracted_file1.exists(), "file1.txt not extracted");
    assert!(extracted_file2.exists(), "file2.txt not extracted");

    let content1 = fs::read_to_string(&extracted_file1).unwrap();
    let content2 = fs::read_to_string(&extracted_file2).unwrap();

    assert_eq!(content1, "Hello, file 1!");
    assert_eq!(content2, "Hello, file 2!");
}
