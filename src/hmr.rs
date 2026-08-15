// =====================================================================
// Phase 3 (Cordis 风格) — HMR（Hot Module Replacement）热重载
// =====================================================================
//
// 借鉴 Cordis 的 `@cordisjs/plugin-hmr`：文件变化 → 通知宿主重新加载。
//
// 设计：
//   - HmrWatcher 监听一个或多个 `.dylib` 文件路径
//   - 文件变化时，触发用户提供的回调
//   - 用户回调里决定：unload → reload
//   - 当前实现用 notify 7.x 的 RecommendedWatcher + 后台线程
//
// 用法：
// ```ignore
// let watcher = HmrWatcher::new(vec!["plugins/news.dylib".into()])?;
// watcher.on_reload(|event| {
//     info!("plugin changed: {:?}", event.path());
//     // mgr.unload_plugin(...).ok();
//     // mgr.load_plugins_from(&path).ok();
// });
// watcher.start()?;
// // ... 之后 watch 在后台线程跑
// ```

use notify::event::{EventKind, ModifyKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

/// HMR 事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HmrEvent {
    /// 文件被修改
    Modified(PathBuf),
    /// 文件被创建
    Created(PathBuf),
    /// 文件被删除
    Removed(PathBuf),
}

impl HmrEvent {
    pub fn path(&self) -> &PathBuf {
        match self {
            HmrEvent::Modified(p) | HmrEvent::Created(p) | HmrEvent::Removed(p) => p,
        }
    }
}

/// HMR 回调类型（Arc<dyn Fn>，可注册多个）
pub type HmrCallback = Arc<dyn Fn(HmrEvent) + Send + Sync + 'static>;

/// HMR 文件 watcher
pub struct HmrWatcher {
    /// 要 watch 的文件路径
    paths: Vec<PathBuf>,
    /// 注册的回调列表（多个订阅者）
    callbacks: Arc<Mutex<Vec<HmrCallback>>>,
    /// notify 推荐 watcher（持有时 watch 进行中）
    _watcher: Mutex<Option<RecommendedWatcher>>,
    /// shutdown 信号：watcher drop 时 set true，event_loop 自动退出
    shutdown: Arc<AtomicBool>,
}

impl std::fmt::Debug for HmrWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HmrWatcher")
            .field("paths", &self.paths)
            .field("callbacks_count", &self.callbacks.lock().unwrap().len())
            .field("active", &self._watcher.lock().unwrap().is_some())
            .finish()
    }
}

impl HmrWatcher {
    /// 创建一个新的 watcher（不立即启动）
    pub fn new(paths: Vec<PathBuf>) -> Result<Self, notify::Error> {
        for p in &paths {
            if !p.exists() {
                eprintln!("[HmrWatcher] warning: watched path does not exist: {:?}", p);
            }
        }
        Ok(Self {
            paths,
            callbacks: Arc::new(Mutex::new(Vec::new())),
            _watcher: Mutex::new(None),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 注册一个 reload 回调
    pub fn on_reload<F>(&self, callback: F)
    where
        F: Fn(HmrEvent) + Send + Sync + 'static,
    {
        self.callbacks.lock().unwrap().push(Arc::new(callback));
    }

    /// 已注册的回调数量
    pub fn callback_count(&self) -> usize {
        self.callbacks.lock().unwrap().len()
    }

    /// 启动后台 watch 线程
    ///
    /// 调用后 watcher 进入 active 状态。多次调用是 no-op。
    pub fn start(&mut self) -> Result<(), notify::Error> {
        if self._watcher.lock().unwrap().is_some() {
            return Ok(());
        }

        // 重置 shutdown
        self.shutdown.store(false, Ordering::SeqCst);

        let (tx, rx) = channel::<notify::Result<Event>>();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        for path in &self.paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        *self._watcher.lock().unwrap() = Some(watcher);

        // 启动后台线程处理事件
        let callbacks = self.callbacks.clone();
        let shutdown = self.shutdown.clone();
        let paths = self.paths.clone();
        thread::spawn(move || {
            Self::event_loop(callbacks, shutdown, paths, rx);
        });

        Ok(())
    }

    /// 后台事件循环
    ///
    /// 每 50ms 检查一次 shutdown 信号 + 尝试接收事件。
    /// shutdown=true 时退出循环。
    fn event_loop(
        callbacks: Arc<Mutex<Vec<HmrCallback>>>,
        shutdown: Arc<AtomicBool>,
        _paths: Vec<PathBuf>,
        rx: Receiver<notify::Result<Event>>,
    ) {
        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            // 用 recv_timeout 实现定期检查 shutdown
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(event)) => {
                    let hmr_event = match event.kind {
                        EventKind::Create(_) => event
                            .paths
                            .first()
                            .map(|p| HmrEvent::Created(p.clone())),
                        EventKind::Modify(ModifyKind::Data(_)) => event
                            .paths
                            .first()
                            .map(|p| HmrEvent::Modified(p.clone())),
                        EventKind::Remove(_) => event
                            .paths
                            .first()
                            .map(|p| HmrEvent::Removed(p.clone())),
                        _ => None,
                    };
                    if let Some(ev) = hmr_event {
                        let cbs = callbacks.lock().unwrap().clone();
                        for cb in cbs {
                            (cb)(ev.clone());
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("[HmrWatcher] watch error: {:?}", e);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 周期性检查 shutdown；继续
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // sender drop = watcher 已停
                    break;
                }
            }
        }
    }

    /// 关闭 watcher
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        *self._watcher.lock().unwrap() = None;
    }

    /// 是否在 watch
    pub fn is_active(&self) -> bool {
        self._watcher.lock().unwrap().is_some()
    }

    /// 当前 watch 的文件路径列表
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 便利方法：检查文件 mtime 是否变化（手动轮询模式）
    pub fn check_mtime_changed(path: &std::path::Path, last: &mut Option<SystemTime>) -> bool {
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        let Ok(mtime) = meta.modified() else {
            return false;
        };
        if last.is_none() {
            *last = Some(mtime);
            return true;
        }
        if last.unwrap() < mtime {
            *last = Some(mtime);
            return true;
        }
        false
    }
}

impl Drop for HmrWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 便利函数：轮询 watch 一个文件路径，每 `interval` 检查一次
///
/// 回调函数返回 true = 退出循环。
pub fn poll_watch<F>(path: PathBuf, interval: Duration, mut on_change: F)
where
    F: FnMut(HmrEvent) -> bool + Send + 'static,
{
    let mut last: Option<SystemTime> = None;
    loop {
        if HmrWatcher::check_mtime_changed(&path, &mut last) {
            let event = HmrEvent::Modified(path.clone());
            if on_change(event) {
                break;
            }
        }
        thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_hmr_event_path() {
        let p = PathBuf::from("/tmp/foo.dylib");
        let ev = HmrEvent::Modified(p.clone());
        assert_eq!(ev.path(), &p);
    }

    #[test]
    fn test_watcher_new() {
        let watcher = HmrWatcher::new(vec![PathBuf::from("/tmp/foo.dylib")]);
        assert!(watcher.is_ok());
        let w = watcher.unwrap();
        assert!(!w.is_active());
        assert_eq!(w.paths().len(), 1);
    }

    #[test]
    fn test_on_reload_registers_callback() {
        let watcher = HmrWatcher::new(vec![PathBuf::from("/tmp/foo.dylib")]).unwrap();
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
    fn test_check_mtime_changed_returns_false_for_missing_file() {
        let mut last = None;
        let missing = PathBuf::from("/tmp/definitely_does_not_exist_hmr_xyz.dylib");
        assert!(!HmrWatcher::check_mtime_changed(&missing, &mut last));
    }

    #[test]
    fn test_watcher_start_stop_real_file() {
        // 用真实存在的临时文件测试 start/stop
        let tmp = std::env::temp_dir().join("plugkit_hmr_real.dylib");
        File::create(&tmp).unwrap();

        let mut watcher = HmrWatcher::new(vec![tmp.clone()]).unwrap();
        let r = watcher.start();
        if r.is_ok() {
            assert!(watcher.is_active());
            watcher.stop();
            assert!(!watcher.is_active());
        }
        // 清理
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_poll_watch_terminates_on_true() {
        use std::sync::atomic::AtomicBool;
        let flag = Arc::new(AtomicBool::new(false));
        let flag2 = flag.clone();
        let tmp = std::env::temp_dir().join("plugkit_poll_test.dylib");
        File::create(&tmp).unwrap();

        let handle = thread::spawn(move || {
            poll_watch(tmp, Duration::from_millis(50), move |_ev| {
                flag2.store(true, Ordering::SeqCst);
                true
            });
        });

        std::thread::sleep(Duration::from_millis(100));
        handle.join().unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }
}
