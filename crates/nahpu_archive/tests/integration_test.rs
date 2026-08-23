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

/// Builds two files below a shared parent and returns `(parent, files, total_bytes)`.
fn sample_tree(root: &std::path::Path) -> (std::path::PathBuf, Vec<std::path::PathBuf>, u64) {
    let base = root.join("source");
    let nested = base.join("media");
    fs::create_dir_all(&nested).unwrap();
    let first = base.join("nahpu.sqlite3");
    let second = nested.join("roost.jpg");
    fs::write(&first, vec![b'a'; 4096]).unwrap();
    fs::write(&second, vec![b'b'; 8192]).unwrap();
    (base, vec![first, second], 4096 + 8192)
}

fn assert_monotonic(snapshots: &[nahpu_archive::progress::ArchiveProgress]) {
    for pair in snapshots.windows(2) {
        assert!(
            pair[1].entries_done >= pair[0].entries_done,
            "entry count went backwards: {pair:?}"
        );
        assert!(
            pair[1].bytes_done >= pair[0].bytes_done,
            "byte count went backwards: {pair:?}"
        );
    }
}

#[test]
fn zip_write_reports_every_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (base, files, total_bytes) = sample_tree(temp_dir.path());
    let output = temp_dir.path().join("archive.zip");

    let mut snapshots = Vec::new();
    ZipArchive::new(&base, None, &output, &files)
        .write_with_progress(&mut |progress| snapshots.push(progress))
        .expect("Failed to write zip archive");

    assert_monotonic(&snapshots);
    let first = snapshots.first().expect("no progress reported");
    assert_eq!(first.entries_total, 2);
    assert_eq!(first.bytes_total, total_bytes);
    assert!(
        snapshots
            .iter()
            .any(|snapshot| snapshot.current_path == "media/roost.jpg"),
        "the archive-relative path was never reported: {snapshots:?}"
    );

    let last = snapshots.last().unwrap();
    assert_eq!(last.entries_done, 2);
    assert_eq!(last.entries_done, last.entries_total);
    assert_eq!(last.bytes_done, total_bytes);
    assert_eq!(last.bytes_done, last.bytes_total);
}

#[test]
fn tar_gzip_write_reports_every_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (base, files, total_bytes) = sample_tree(temp_dir.path());
    let output = temp_dir.path().join("archive.tar.gz");

    let mut snapshots = Vec::new();
    TarGzipArchive::new(&base, &output, &files)
        .write_with_progress(&mut |progress| snapshots.push(progress))
        .expect("Failed to write tar.gz archive");

    assert_monotonic(&snapshots);
    let last = snapshots.last().expect("no progress reported");
    assert_eq!(last.entries_done, 2);
    assert_eq!(last.entries_done, last.entries_total);
    assert_eq!(last.bytes_done, total_bytes);
    assert_eq!(last.bytes_done, last.bytes_total);
}

#[test]
fn zip_extract_reports_every_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (base, files, total_bytes) = sample_tree(temp_dir.path());
    let output = temp_dir.path().join("archive.zip");
    ZipArchive::new(&base, None, &output, &files)
        .write()
        .unwrap();

    let extract_dir = temp_dir.path().join("extracted");
    let mut snapshots = Vec::new();
    ZipExtractor::new(&output, &extract_dir)
        .extract_with_progress(&mut |progress| snapshots.push(progress))
        .expect("Failed to extract zip archive");

    assert_monotonic(&snapshots);
    let last = snapshots.last().expect("no progress reported");
    assert_eq!(last.entries_done, 2);
    assert_eq!(last.bytes_done, total_bytes);
    assert!(extract_dir.join("media/roost.jpg").exists());
}

#[test]
fn tar_gzip_extract_reports_running_counts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (base, files, total_bytes) = sample_tree(temp_dir.path());
    let output = temp_dir.path().join("archive.tar.gz");
    TarGzipArchive::new(&base, &output, &files).write().unwrap();

    let extract_dir = temp_dir.path().join("extracted");
    let mut snapshots = Vec::new();
    TarGzipExtractor::new(&output, &extract_dir)
        .extract_with_progress(&mut |progress| snapshots.push(progress))
        .expect("Failed to extract tar.gz archive");

    assert_monotonic(&snapshots);
    // A tar stream has no index, so totals are only reconciled by the final snapshot.
    let last = snapshots.last().expect("no progress reported");
    assert_eq!(last.entries_done, 2);
    assert_eq!(last.entries_done, last.entries_total);
    assert_eq!(last.bytes_done, total_bytes);
    assert!(extract_dir.join("media/roost.jpg").exists());
}

#[test]
fn gzip_reports_progress_in_both_directions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let input = temp_dir.path().join("nahpu-project.json");
    fs::write(&input, vec![b'{'; 32768]).unwrap();
    let compressed = temp_dir.path().join("nahpu-project.json.gz");

    let mut compress_snapshots = Vec::new();
    gzip::compress_with_progress(&input, &compressed, &mut |progress| {
        compress_snapshots.push(progress)
    })
    .expect("Failed to gzip input");

    assert_monotonic(&compress_snapshots);
    let last = compress_snapshots.last().expect("no progress reported");
    assert_eq!(last.bytes_done, 32768);
    assert_eq!(last.entries_done, 1);

    let extracted = temp_dir.path().join("output.json");
    let mut extract_snapshots = Vec::new();
    gzip::decompress_with_progress(&compressed, &extracted, &mut |progress| {
        extract_snapshots.push(progress)
    })
    .expect("Failed to gunzip input");

    assert_monotonic(&extract_snapshots);
    assert_eq!(extract_snapshots.last().unwrap().entries_done, 1);
    assert_eq!(fs::read(&input).unwrap(), fs::read(&extracted).unwrap());
}
