use std::fs;

use lazyssh::config::models::{Auth, Server};
use lazyssh::sftp::LocalFs;
use lazyssh::tui::sftp_browser::{format_size, SftpState, TransferProgress};

fn test_server() -> Server {
    Server {
        name: "test".into(),
        host: "test".into(),
        port: 22,
        user: "user".into(),
        auth: Auth::Key {
            path: "~/.ssh/id_rsa".into(),
            passphrase: None,
        },
        tags: vec![],
        pinned: false,
        last_connected: None,
        connection_count: 0,
            bookmarks: vec![],
            agent_forwarding: false,
            proxy_jump: None,
            log_enabled: false,
    }
}

// --- TransferProgress ---

#[test]
fn test_transfer_progress_percentage_partial() {
    let p = TransferProgress {
        file_name: "f.txt".into(),
        bytes_done: 50,
        bytes_total: 100,
        is_upload: true,
        start_time: std::time::Instant::now(),
    };
    assert_eq!(p.percentage(), 50);
    assert!(!p.is_complete());
}

#[test]
fn test_transfer_progress_percentage_zero_total() {
    let p = TransferProgress {
        file_name: "f.txt".into(),
        bytes_done: 0,
        bytes_total: 0,
        is_upload: true,
        start_time: std::time::Instant::now(),
    };
    assert_eq!(p.percentage(), 0);
    assert!(p.is_complete());
}

#[test]
fn test_transfer_progress_complete() {
    let p = TransferProgress {
        file_name: "f.txt".into(),
        bytes_done: 100,
        bytes_total: 100,
        is_upload: false,
        start_time: std::time::Instant::now(),
    };
    assert_eq!(p.percentage(), 100);
    assert!(p.is_complete());
}

#[test]
fn test_transfer_progress_is_upload() {
    let up = TransferProgress {
        file_name: "up".into(),
        bytes_done: 0,
        bytes_total: 10,
        is_upload: true,
        start_time: std::time::Instant::now(),
    };
    assert!(up.is_upload);
    let down = TransferProgress {
        file_name: "down".into(),
        bytes_done: 0,
        bytes_total: 10,
        is_upload: false,
        start_time: std::time::Instant::now(),
    };
    assert!(!down.is_upload);
}

// --- SftpState ---

#[test]
fn test_sftp_state_new() {
    let state = SftpState::new(test_server());
    assert_eq!(state.remote_path, "/home/user");
    assert_eq!(state.focus_side, lazyssh::tui::sftp_browser::Side::Local);
    assert_eq!(state.status, "Conectando...");
    assert!(state.remote_files.is_empty());
    assert!(!state.is_transferring);
}

#[test]
fn test_sftp_state_refresh_remote() {
    let mut state = SftpState::new(test_server());
    let files = vec![
        lazyssh::sftp::FileInfo {
            name: "a".into(),
            is_dir: true,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "b".into(),
            is_dir: false,
        permissions: None,
            size: 100,
        },
    ];
    state.refresh_remote(files);
    assert_eq!(state.remote_files.len(), 2);
    assert_eq!(state.remote_selected, 0);
}

#[test]
fn test_sftp_state_next_item_local() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![
        lazyssh::sftp::FileInfo {
            name: "a".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "b".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
    ];
    state.next_item();
    assert_eq!(state.local_selected, 1);
    state.next_item();
    assert_eq!(state.local_selected, 1, "should not go past last");
}

#[test]
fn test_sftp_state_next_item_remote() {
    let mut state = SftpState::new(test_server());
    state.focus_side = lazyssh::tui::sftp_browser::Side::Remote;
    state.remote_files = vec![
        lazyssh::sftp::FileInfo {
            name: "x".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "y".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
    ];
    state.next_item();
    assert_eq!(state.remote_selected, 1);
}

#[test]
fn test_sftp_state_previous_item() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![
        lazyssh::sftp::FileInfo {
            name: "a".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "b".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
    ];
    state.local_selected = 1;
    state.previous_item();
    assert_eq!(state.local_selected, 0);
    state.previous_item();
    assert_eq!(state.local_selected, 0, "should not go below 0");
}

#[test]
fn test_sftp_state_enter_local_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt"), "data").unwrap();

    let mut state = SftpState::new(test_server());
    state.local = LocalFs {
        current_dir: dir.path().to_path_buf(),
    };
    state.local_files = vec![lazyssh::sftp::FileInfo {
        name: "file.txt".into(),
        is_dir: false,
        permissions: None,
        size: 4,
    }];

    // Enter a file should fail (not a directory)
    let result = state.enter_directory();
    assert!(result.is_err(), "Entering a file should fail");
}

#[test]
fn test_sftp_state_enter_remote_returns_path() {
    let mut state = SftpState::new(test_server());
    state.focus_side = lazyssh::tui::sftp_browser::Side::Remote;
    state.remote_files = vec![lazyssh::sftp::FileInfo {
        name: "subdir".into(),
        is_dir: true,
        permissions: None,
        size: 0,
    }];
    state.remote_selected = 0;

    let result = state.enter_directory();
    assert!(result.is_ok());
    assert_eq!(state.remote_path, "/home/user/subdir");
}

#[test]
fn test_sftp_state_go_parent_local() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut state = SftpState::new(test_server());
    state.local = LocalFs {
        current_dir: dir.path().to_path_buf(),
    };
    state.go_parent().unwrap();
    assert_ne!(state.local.current_dir(), dir.path());
}

#[test]
fn test_sftp_state_go_parent_remote_root() {
    let mut state = SftpState::new(test_server());
    state.focus_side = lazyssh::tui::sftp_browser::Side::Remote;
    state.remote_path = "/".to_string();
    let result = state.go_parent();
    assert!(result.is_ok());
    assert_eq!(state.remote_path, "/");
}

#[test]
fn test_sftp_state_go_parent_remote_subdir() {
    let mut state = SftpState::new(test_server());
    state.focus_side = lazyssh::tui::sftp_browser::Side::Remote;
    state.remote_path = "/home/user/docs".to_string();
    state.go_parent().unwrap();
    assert_eq!(state.remote_path, "/home/user");
}

#[test]
fn test_sftp_state_toggle_focus() {
    let mut state = SftpState::new(test_server());
    assert_eq!(
        state.focus_side,
        lazyssh::tui::sftp_browser::Side::Local
    );
    state.toggle_focus();
    assert_eq!(
        state.focus_side,
        lazyssh::tui::sftp_browser::Side::Remote
    );
    state.toggle_focus();
    assert_eq!(
        state.focus_side,
        lazyssh::tui::sftp_browser::Side::Local
    );
}

#[test]
fn test_sftp_state_toggle_select_current() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![
        lazyssh::sftp::FileInfo {
            name: "a".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "b".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
    ];
    state.toggle_select_current();
    assert_eq!(state.local_selected_files.len(), 1);
    assert!(state.local_selected_files.contains(&0));

    state.toggle_select_current();
    assert!(state.local_selected_files.is_empty());
}

#[test]
fn test_sftp_state_select_all_local() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![
        lazyssh::sftp::FileInfo {
            name: "a".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "b".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
        lazyssh::sftp::FileInfo {
            name: "c".into(),
            is_dir: false,
        permissions: None,
            size: 0,
        },
    ];
    state.select_all_current();
    assert_eq!(state.local_selected_files.len(), 3);
}

#[test]
fn test_sftp_state_select_all_remote() {
    let mut state = SftpState::new(test_server());
    state.focus_side = lazyssh::tui::sftp_browser::Side::Remote;
    state.remote_files = vec![lazyssh::sftp::FileInfo {
        name: "x".into(),
        is_dir: false,
        permissions: None,
        size: 0,
    }];
    state.select_all_current();
    assert_eq!(state.remote_selected_files.len(), 1);
}

#[test]
fn test_sftp_state_clear_selection() {
    let mut state = SftpState::new(test_server());
    state.local_selected_files = vec![0, 1, 2];
    state.clear_selection();
    assert!(state.local_selected_files.is_empty());
}

#[test]
fn test_sftp_state_get_selected_files() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![
        lazyssh::sftp::FileInfo {
            name: "a".into(),
            is_dir: false,
        permissions: None,
            size: 1,
        },
        lazyssh::sftp::FileInfo {
            name: "b".into(),
            is_dir: false,
        permissions: None,
            size: 2,
        },
    ];
    state.local_selected_files = vec![0, 1];
    let selected = state.get_selected_files();
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].size, 1);
    assert_eq!(selected[1].size, 2);
}

#[test]
fn test_sftp_state_transfer_lifecycle() {
    let mut state = SftpState::new(test_server());
    state.start_transfer("file.bin".into(), 1000, true);
    assert!(state.is_transferring);
    assert_eq!(
        state.transfer_progress.as_ref().unwrap().percentage(),
        0
    );

    state.update_transfer_progress(500);
    assert!(state.is_transferring);
    assert_eq!(
        state.transfer_progress.as_ref().unwrap().percentage(),
        50
    );

    state.update_transfer_progress(1000);
    assert!(
        !state.is_transferring,
        "complete transfer should clear is_transferring"
    );

    state.finish_transfer();
    assert!(state.transfer_progress.is_none());
    assert!(state.local_selected_files.is_empty());
}

#[test]
fn test_sftp_state_next_item_empty_no_crash() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![];
    state.next_item();
    assert_eq!(state.local_selected, 0);
}

#[test]
fn test_sftp_state_previous_item_empty_no_crash() {
    let mut state = SftpState::new(test_server());
    state.local_files = vec![];
    state.previous_item();
    assert_eq!(state.local_selected, 0);
}

// --- format_size ---

#[test]
fn test_format_size_bytes() {
    assert_eq!(format_size(0), "0B");
    assert_eq!(format_size(512), "512B");
    assert_eq!(format_size(1023), "1023B");
}

#[test]
fn test_format_size_kilobytes() {
    assert_eq!(format_size(1024), "1.0K");
    assert_eq!(format_size(1536), "1.5K");
    assert_eq!(format_size(1024 * 1023), "1023.0K");
}

#[test]
fn test_format_size_megabytes() {
    assert_eq!(format_size(1024 * 1024), "1.0M");
    assert_eq!(format_size(2 * 1024 * 1024), "2.0M");
}

#[test]
fn test_format_size_gigabytes() {
    let gb = 1024u64 * 1024 * 1024;
    assert_eq!(format_size(gb), "1.0G");
    assert_eq!(format_size(3 * gb), "3.0G");
}
