use std::fs;
use tempfile::TempDir;

use lazyssh::sftp::LocalFs;

fn create_local_fs_in(dir: &TempDir) -> LocalFs {
    LocalFs {
        current_dir: dir.path().to_path_buf(),
    }
}

#[test]
fn test_list_empty_dir() {
    let dir = TempDir::new().unwrap();
    let fs = create_local_fs_in(&dir);
    let entries = fs.list().unwrap();
    assert!(entries.is_empty(), "Empty dir should return no entries");
}

#[test]
fn test_list_with_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "hello").unwrap();
    fs::write(dir.path().join("b.txt"), "world").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let fs = create_local_fs_in(&dir);
    let mut entries = fs.list().unwrap();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].name, "a.txt");
    assert!(!entries[0].is_dir);
    assert_eq!(entries[2].name, "subdir");
    assert!(entries[2].is_dir);
}

#[test]
fn test_cd_relative() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();
    fs::write(dir.path().join("subdir/file.txt"), "data").unwrap();

    let mut fs = create_local_fs_in(&dir);
    fs.cd("subdir").unwrap();
    assert!(fs.current_dir().ends_with("subdir"));

    let entries = fs.list().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "file.txt");
}

#[test]
fn test_cd_absolute() {
    let dir = TempDir::new().unwrap();
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let mut fs = create_local_fs_in(&dir);
    fs.cd(subdir.to_str().unwrap()).unwrap();
    assert!(fs.current_dir().ends_with("subdir"));
}

#[test]
fn test_cd_nonexistent() {
    let dir = TempDir::new().unwrap();
    let mut fs = create_local_fs_in(&dir);
    let result = fs.cd("nonexistent");
    assert!(result.is_err(), "cd to nonexistent dir should fail");
}

#[test]
fn test_cd_file_fails() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt"), "data").unwrap();

    let mut fs = create_local_fs_in(&dir);
    let result = fs.cd("file.txt");
    assert!(result.is_err(), "cd to file should fail");
}

#[test]
fn test_current_dir() {
    let dir = TempDir::new().unwrap();
    let fs = create_local_fs_in(&dir);
    assert_eq!(fs.current_dir(), dir.path());
}

#[test]
fn test_download() {
    let dir = TempDir::new().unwrap();
    let fs = create_local_fs_in(&dir);
    fs.download("remote content", "output.txt").unwrap();

    let content = fs::read_to_string(dir.path().join("output.txt")).unwrap();
    assert_eq!(content, "remote content");
}

#[test]
fn test_get_file_size() {
    let dir = TempDir::new().unwrap();
    let data = "hello world";
    fs::write(dir.path().join("data.txt"), data).unwrap();

    let fs = create_local_fs_in(&dir);
    let size = fs.get_file_size("data.txt").unwrap();
    assert_eq!(size, data.len() as u64);
}

#[test]
fn test_get_full_path() {
    let dir = TempDir::new().unwrap();
    let fs = create_local_fs_in(&dir);
    let full = fs.get_full_path("xyz.txt");
    assert!(full.ends_with("xyz.txt"));
    assert!(full.starts_with(dir.path().to_str().unwrap()));
}

#[test]
fn test_new_uses_home() {
    let fs = LocalFs::new();
    assert!(fs.current_dir().exists(), "Home directory should exist");
}
