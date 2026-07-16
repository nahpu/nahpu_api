use nahpu_archive::archive::{ZipArchive, ZipExtractor};
use nahpu_archive::gzip;
use nahpu_archive::tar_gzip::{TarGzipArchive, TarGzipExtractor};
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

#[test]
fn gzip_round_trip() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let input = temp_dir.path().join("input.csv");
    let compressed = temp_dir.path().join("input.csv.gz");
    let extracted = temp_dir.path().join("output.csv");
    fs::write(
        &input,
        "occurrenceID,basisOfRecord\nocc-1,PreservedSpecimen\n",
    )
    .expect("Failed to write input");

    gzip::compress(&input, &compressed).expect("Failed to gzip input");
    gzip::decompress(&compressed, &extracted).expect("Failed to gunzip input");

    assert_eq!(fs::read(&input).unwrap(), fs::read(&extracted).unwrap());
}

#[test]
fn tar_gzip_round_trip() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let input_dir = temp_dir.path().join("input");
    let nested_dir = input_dir.join("nested");
    fs::create_dir_all(&nested_dir).expect("Failed to create input");
    let first = input_dir.join("datapackage.json");
    let second = nested_dir.join("records.csv");
    fs::write(&first, "{}").expect("Failed to write descriptor");
    fs::write(&second, "id\n1\n").expect("Failed to write data");

    let archive_path = temp_dir.path().join("package.tar.gz");
    TarGzipArchive::new(&input_dir, &archive_path, &[first.clone(), second.clone()])
        .write()
        .expect("Failed to create tar.gz");

    let output_dir = temp_dir.path().join("output");
    TarGzipExtractor::new(&archive_path, &output_dir)
        .extract()
        .expect("Failed to extract tar.gz");

    assert_eq!(
        fs::read_to_string(output_dir.join("datapackage.json")).unwrap(),
        "{}"
    );
    assert_eq!(
        fs::read_to_string(output_dir.join("nested/records.csv")).unwrap(),
        "id\n1\n"
    );
}
