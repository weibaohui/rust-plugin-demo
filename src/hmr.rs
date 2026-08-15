//! Re-export of cordis-rs-hmr types.
//!
//! 老代码 `plugkit::hmr::HmrWatcher` 仍可用。
//! 新代码推荐 `use cordis_rs_hmr::*`。

pub use cordis_rs_hmr::{HmrEvent, HmrWatcher, HmrCallback};

// =====================================================================
// 集成测试 —— 验证 cordis-rs 类型在 plugkit 上下文正常工作
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::Duration;

    #[test]
    fn test_hmr_event_path() {
        let p = std::path::PathBuf::from("/tmp/foo.dylib");
        let ev = HmrEvent::Modified(p.clone());
        assert_eq!(ev.path(), &p);
    }

    #[test]
    fn test_watcher_new() {
        let watcher = HmrWatcher::new(vec![std::path::PathBuf::from("/tmp/foo.dylib")]);
        assert!(watcher.is_ok());
        let w = watcher.unwrap();
        assert!(!w.is_active());
    }

    #[test]
    fn test_on_reload_registers_callback() {
        let watcher = HmrWatcher::new(vec![std::path::PathBuf::from("/tmp/foo.dylib")]).unwrap();
        watcher.on_reload(|_event| {});
        watcher.on_reload(|_event| {});
        assert_eq!(watcher.callback_count(), 2);
    }

    #[test]
    fn test_check_mtime_changed_detects_modification() {
        let tmp = std::env::temp_dir().join("plugkit_hmr_test.dylib");
        let mut f = File::create(&tmp).unwrap();
        f.write_all(b"v1").unwrap();
        f.sync_all().unwrap();
        drop(f);

        let mut last = None;
        assert!(HmrWatcher::check_mtime_changed(&tmp, &mut last));
        std::thread::sleep(Duration::from_millis(50));
        let mut f = File::create(&tmp).unwrap();
        f.write_all(b"v2").unwrap();
        f.sync_all().unwrap();
        drop(f);
        assert!(HmrWatcher::check_mtime_changed(&tmp, &mut last));
        assert!(!HmrWatcher::check_mtime_changed(&tmp, &mut last));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_check_mtime_changed_returns_false_for_missing() {
        let mut last = None;
        let missing = std::path::PathBuf::from("/tmp/missing.dylib");
        assert!(!HmrWatcher::check_mtime_changed(&missing, &mut last));
    }

    #[test]
    fn test_watcher_start_stop_real_file() {
        let tmp = std::env::temp_dir().join("plugkit_hmr_real.dylib");
        File::create(&tmp).unwrap();
        let mut watcher = HmrWatcher::new(vec![tmp.clone()]).unwrap();
        if watcher.start().is_ok() {
            assert!(watcher.is_active());
            watcher.stop();
            assert!(!watcher.is_active());
        }
        std::fs::remove_file(&tmp).ok();
    }
}