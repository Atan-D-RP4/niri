//! File modification watcher.

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};
use std::{io, thread};

use niri_config::{Config, ConfigParseResult, ConfigPath};
use smithay::reexports::calloop::channel::SyncSender;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::{Interest, Mode, PostAction};

use crate::niri::State;

/// inotify events to watch for in the config directories (Linux).
///
/// We watch the directories rather than the files themselves, so that the config file (and the
/// include files) can be created, removed, or renamed freely. The `IN_IGNORED`, `IN_DELETE_SELF`,
/// `IN_MOVE_SELF`, and `IN_Q_OVERFLOW` events are delivered automatically regardless of the mask.
///
/// Notably `IN_MODIFY` is not watched: every completed write ends in `IN_CLOSE_WRITE`, and atomic
/// editor saves end in `IN_MOVED_TO`, so `IN_MODIFY` would only add noise from partial writes.
#[cfg(target_os = "linux")]
const DIR_WATCH_MASK: u32 = libc::IN_CREATE
    | libc::IN_DELETE
    | libc::IN_MOVED_TO
    | libc::IN_MOVED_FROM
    | libc::IN_CLOSE_WRITE
    | libc::IN_ATTRIB;

/// kqueue event flags for watching config directories (non-Linux).
///
/// `NOTE_WRITE` fires when directory entries change (create/delete/rename — catches atomic saves).
/// `NOTE_DELETE`/`NOTE_RENAME` fire when the directory itself is deleted or renamed.
/// `NOTE_ATTRIB` fires when directory attributes change.
///
/// Unlike inotify, kqueue directory watches do *not* fire for content writes to files within the
/// directory. In-place file writes are caught by the timer safety net instead.
#[cfg(not(target_os = "linux"))]
const KQ_DIR_WATCH_FLAGS: u32 =
    libc::NOTE_WRITE | libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_ATTRIB;

/// How often the timer re-arms the config directory watches and re-checks the config.
///
/// This is a safety net for cases where trigger events are missed, the watched directories don't
/// exist yet, or the change doesn't produce an event in any watched directory (e.g. a config
/// directory symlink being retargeted). It runs at the same cadence as the old polling watcher,
/// so those blind spots are caught with the same latency as before.
const TIMER_INTERVAL: Duration = Duration::from_millis(500);

/// Messages sent to the watcher thread.
enum WatcherMsg {
    /// Reload the config, optionally switching to a new config path.
    Reload(Option<String>),
    /// Check whether the config changed.
    Check,
}

pub struct Watcher {
    load_config: mpsc::Sender<WatcherMsg>,
}

struct WatcherInner {
    /// The paths we're watching.
    path: ConfigPath,

    /// Last observed props of the watched file.
    last_props: Option<Props>,

    /// Last observed props for included files.
    includes: HashMap<PathBuf, Option<Props>>,
}

/// Properties of the watched file.
///
/// Equality on this means the file did not change.
#[derive(Debug, PartialEq, Eq)]
struct Props {
    /// Modification time of the watched file.
    mtime: SystemTime,

    /// Canonical form of the watched path.
    ///
    /// We store the absolute path in addition to mtime to account for symlinked configs where the
    /// symlink target may change without mtime. This is common on nix where everything is a
    /// symlink to /nix/store, which keeps no mtime (= 1970-01-01).
    canonical: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
enum CheckResult {
    Missing,
    Unchanged,
    Changed,
}

impl Watcher {
    pub fn new(
        path: ConfigPath,
        includes: Vec<PathBuf>,
        mut process: impl FnMut(&ConfigPath) -> ConfigParseResult<Config, ()> + Send + 'static,
        changed: SyncSender<Result<Config, ()>>,
        trigger_fd: i32,
    ) -> Self {
        let (load_config, load_config_rx) = mpsc::channel();

        thread::Builder::new()
            .name(format!("Filesystem Watcher for {path:?}"))
            .spawn(move || {
                let mut inner = WatcherInner::new(path, includes);

                // Directory watches for the include files. Re-armed after every config parse,
                // since the set of includes is only known after the config is parsed.
                let mut include_watches: HashMap<PathBuf, i32> = HashMap::new();

                loop {
                    let mut should_load = match load_config_rx.recv() {
                        Ok(WatcherMsg::Reload(path)) => {
                            if let Some(path) = path {
                                inner = WatcherInner::new(
                                    ConfigPath::Explicit(PathBuf::from(path)),
                                    Vec::new(),
                                );

                                // The previous config's includes no longer apply; drop their
                                // directory watches. They will be re-armed after the next parse.
                                #[cfg(target_os = "linux")]
                                for wd in include_watches.drain().map(|(_, wd)| wd) {
                                    unsafe { libc::inotify_rm_watch(trigger_fd, wd) };
                                }
                                #[cfg(not(target_os = "linux"))]
                                for fd in include_watches.drain().map(|(_, fd)| fd) {
                                    unsafe { libc::close(fd) };
                                }
                            }
                            true
                        }
                        Ok(WatcherMsg::Check) => false,
                        Err(_) => break,
                    };

                    match inner.check() {
                        CheckResult::Missing => continue,
                        CheckResult::Unchanged => (),
                        CheckResult::Changed => {
                            trace!("config file changed");
                            should_load = true;
                        }
                    }

                    if should_load {
                        let res = process(&inner.path);

                        if let Err(err) = changed.send(res.config) {
                            warn!("error sending change notification: {err:?}");
                            break;
                        }

                        // There's a bit of time here between reading the config and reading
                        // properties of included files where an included file could change and
                        // remain unnoticed by the watcher. Not sure there's any good way around it
                        // though since we don't know the final set of includes until the config is
                        // parsed.
                        inner.set_includes(res.includes);

                        // Re-arm the directory watches for the included files.
                        update_include_watches(trigger_fd, &inner.includes, &mut include_watches);
                    }
                }

                debug!("exiting watcher thread for {:?}", inner.path);
                unsafe { libc::close(trigger_fd) };
            })
            .unwrap();

        Self { load_config }
    }

    pub fn load_config(&self, path: Option<String>) {
        let _ = self.load_config.send(WatcherMsg::Reload(path));
    }
}

impl Props {
    fn from_path(path: &Path) -> io::Result<Self> {
        let canonical = path.canonicalize()?;
        let mtime = canonical.metadata()?.modified()?;
        Ok(Self { mtime, canonical })
    }

    fn from_config_path(config_path: &ConfigPath) -> io::Result<Self> {
        match config_path {
            ConfigPath::Explicit(path) => Self::from_path(path),
            ConfigPath::Regular {
                user_path,
                system_path,
            } => Self::from_path(user_path).or_else(|_| Self::from_path(system_path)),
        }
    }
}

impl WatcherInner {
    pub fn new(path: ConfigPath, includes: Vec<PathBuf>) -> Self {
        let last_props = Props::from_config_path(&path).ok();

        let mut rv = Self {
            path,
            last_props,
            includes: HashMap::new(),
        };
        rv.set_includes(includes);
        rv
    }

    pub fn check(&mut self) -> CheckResult {
        if let Ok(new_props) = Props::from_config_path(&self.path) {
            if self.last_props.as_ref() != Some(&new_props) {
                self.last_props = Some(new_props);
                CheckResult::Changed
            } else {
                for (path, last_props) in &mut self.includes {
                    let new_props = Props::from_path(path).ok();

                    // If an include goes missing while the main config file is unchanged, we
                    // consider that a change and reload.
                    if *last_props != new_props {
                        return CheckResult::Changed;
                    }
                }

                CheckResult::Unchanged
            }
        } else {
            CheckResult::Missing
        }
    }

    fn set_includes(&mut self, includes: Vec<PathBuf>) {
        self.includes = includes
            .into_iter()
            .map(|path| {
                let props = Props::from_path(&path).ok();
                (path, props)
            })
            .collect();
    }
}

pub fn setup(state: &mut State, config_path: &ConfigPath, includes: Vec<PathBuf>) {
    // Parsing the config actually takes > 20 ms on my beefy machine, so let's do it on the
    // watcher thread.
    let process = |path: &ConfigPath| {
        path.load().map_config_res(|res| {
            res.map_err(|err| {
                warn!("{err:?}");
            })
        })
    };

    let (tx, rx) = calloop::channel::sync_channel(1);
    state
        .niri
        .event_loop
        .insert_source(
            rx,
            |event: calloop::channel::Event<Result<Config, ()>>, _, state| match event {
                calloop::channel::Event::Msg(config) => {
                    let failed = config.is_err();
                    state.reload_config(config);
                    state.ipc_config_loaded(failed);
                }
                calloop::channel::Event::Closed => (),
            },
        )
        .unwrap();

    // Create the platform trigger instance to watch for changes to the config and include files.
    #[cfg(target_os = "linux")]
    let trigger_fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
    #[cfg(target_os = "linux")]
    if trigger_fd < 0 {
        panic!("error initializing inotify: {}", io::Error::last_os_error());
    }

    #[cfg(not(target_os = "linux"))]
    let trigger_fd = unsafe { libc::kqueue() };
    #[cfg(not(target_os = "linux"))]
    if trigger_fd < 0 {
        panic!("error initializing kqueue: {}", io::Error::last_os_error());
    }

    // Watch the config parent directories. We watch the directories (not the files), so that the
    // config file can be created, removed, or renamed freely. Directories that don't exist yet
    // are simply skipped; the timer below will retry.
    let config_parent_dirs = config_parent_dirs(config_path);
    for dir in &config_parent_dirs {
        add_dir_watch(trigger_fd, dir);
    }

    // The watcher thread needs its own fd, so that it doesn't race with the event loop closing
    // the original one at shutdown.
    let watcher_fd = unsafe { libc::dup(trigger_fd) };
    if watcher_fd < 0 {
        panic!(
            "error duplicating trigger fd: {}",
            io::Error::last_os_error()
        );
    }
    let watcher = Watcher::new(config_path.clone(), includes, process, tx, watcher_fd);
    let load_config = watcher.load_config.clone();

    // On any trigger event, ask the watcher thread to check whether the config changed. The
    // watcher thread's check() is the authoritative filter, so we don't over-filter here.
    let generic_tx = load_config.clone();
    state
        .niri
        .event_loop
        .insert_source(
            Generic::new(
                unsafe { OwnedFd::from_raw_fd(trigger_fd) },
                Interest::READ,
                Mode::Level,
            ),
            move |_, _, _| {
                if let Err(err) = drain_trigger_events(trigger_fd, || {
                    let _ = generic_tx.send(WatcherMsg::Check);
                }) {
                    warn!("error reading trigger events: {err:?}");
                }

                Ok(PostAction::Continue)
            },
        )
        .unwrap();

    // As a safety net, periodically re-arm the config directory watches (they may have been
    // removed, or the directories may not have existed yet) and ask the watcher thread to check
    // for changes.
    #[cfg(target_os = "linux")]
    state
        .niri
        .event_loop
        .insert_source(Timer::from_duration(TIMER_INTERVAL), move |_, _, _| {
            // inotify_add_watch() is idempotent: re-adding an already-watched path returns the
            // existing watch descriptor.
            for dir in &config_parent_dirs {
                add_dir_watch(trigger_fd, dir);
            }

            let _ = load_config.send(WatcherMsg::Check);
            TimeoutAction::ToDuration(TIMER_INTERVAL)
        })
        .unwrap();

    // On non-Linux (kqueue), the timer must close and reopen directory fds to re-arm watches,
    // since kqueue watches are tied to file descriptors and are not idempotent.
    #[cfg(not(target_os = "linux"))]
    {
        let mut config_dir_fds: Vec<Option<i32>> =
            config_parent_dirs.iter().map(|_| None).collect();
        // Register initial watches.
        for (i, dir) in config_parent_dirs.iter().enumerate() {
            config_dir_fds[i] = add_dir_watch(trigger_fd, dir);
        }
        state
            .niri
            .event_loop
            .insert_source(Timer::from_duration(TIMER_INTERVAL), move |_, _, _| {
                for (i, dir) in config_parent_dirs.iter().enumerate() {
                    if let Some(fd) = config_dir_fds[i].take() {
                        unsafe { libc::close(fd) };
                    }
                    config_dir_fds[i] = add_dir_watch(trigger_fd, dir);
                }

                let _ = load_config.send(WatcherMsg::Check);
                TimeoutAction::ToDuration(TIMER_INTERVAL)
            })
            .unwrap();
    }

    state.niri.config_file_watcher = Some(watcher);
}

/// Returns the parent directories of the config file(s), deduplicated.
fn config_parent_dirs(config_path: &ConfigPath) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut add = |dir: Option<&Path>| {
        if let Some(dir) = dir {
            if !dirs.iter().any(|d| d == dir) {
                dirs.push(dir.to_path_buf());
            }
        }
    };

    match config_path {
        ConfigPath::Explicit(path) => add(path.parent()),
        ConfigPath::Regular {
            user_path,
            system_path,
        } => {
            add(user_path.parent());
            add(system_path.parent());
        }
    }

    dirs
}

// --- Linux (inotify) ---

/// Adds an inotify directory watch for `dir`, returning the watch descriptor.
///
/// Returns `None` if the directory doesn't exist yet; the caller (or the timer) can retry later.
#[cfg(target_os = "linux")]
fn add_dir_watch(fd: i32, dir: &Path) -> Option<i32> {
    let Ok(c_dir) = CString::new(dir.as_os_str().as_bytes()) else {
        // A path with an embedded NUL byte cannot exist; nothing to watch.
        return None;
    };

    let wd = unsafe { libc::inotify_add_watch(fd, c_dir.as_ptr(), DIR_WATCH_MASK) };
    if wd < 0 {
        // The directory may not exist yet; the timer will retry.
        trace!(
            "error watching directory {dir:?}: {}",
            io::Error::last_os_error()
        );
        return None;
    }

    Some(wd)
}

/// Re-arms the inotify directory watches for the include files.
///
/// Called on the watcher thread after every config parse, since the set of includes is only
/// known after parsing.
#[cfg(target_os = "linux")]
fn update_include_watches(
    trigger_fd: i32,
    includes: &HashMap<PathBuf, Option<Props>>,
    watches: &mut HashMap<PathBuf, i32>,
) {
    // The include parent directories, canonicalized to dedupe symlinked paths.
    let include_dirs: HashSet<PathBuf> = includes
        .keys()
        .filter_map(|path| path.parent())
        .filter_map(|dir| dir.canonicalize().ok())
        .collect();

    // Remove watches for directories that no longer contain include files.
    watches.retain(|dir, wd| {
        if include_dirs.contains(dir) {
            true
        } else {
            unsafe { libc::inotify_rm_watch(trigger_fd, *wd) };
            false
        }
    });

    // Add watches for new include directories. inotify_add_watch() on an already-watched path
    // just returns the existing watch descriptor, so this is safe even if a directory is watched
    // both as a config parent and as an include parent.
    for dir in include_dirs {
        if !watches.contains_key(&dir) {
            if let Some(wd) = add_dir_watch(trigger_fd, &dir) {
                watches.insert(dir, wd);
            }
        }
    }
}

/// Drains all pending inotify events from `fd`, calling `handle` for each event that isn't pure
/// bookkeeping (an `IN_IGNORED`-only event).
///
/// Returns the number of events passed to `handle`.
#[cfg(target_os = "linux")]
fn drain_trigger_events(fd: i32, mut handle: impl FnMut()) -> io::Result<usize> {
    let mut buf = vec![0u8; 4096];
    let mut handled = 0;
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n > 0 {
            let mut offset = 0usize;
            while offset < n as usize {
                let event = unsafe { &*(buf.as_ptr().add(offset) as *const libc::inotify_event) };

                // Advance past this event, including its name.
                offset += size_of::<libc::inotify_event>() + event.len as usize;

                if event.mask != 0 && (event.mask & !libc::IN_IGNORED) != 0 {
                    handle();
                    handled += 1;
                }
            }
        } else if n == 0 {
            // Should never happen, but be safe.
            break;
        } else {
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(libc::EINTR) => continue,
                // EAGAIN means we have drained all pending events. (On Linux
                // EWOULDBLOCK is the same as EAGAIN.)
                Some(libc::EAGAIN) => break,
                _ => return Err(err),
            }
        }
    }

    Ok(handled)
}

// --- Non-Linux (kqueue) ---

/// Opens a directory and registers a kqueue watch for it, returning the open fd.
///
/// Returns `None` if the directory doesn't exist yet; the caller (or the timer) can retry later.
/// Closing the returned fd automatically deregisters the kevent.
#[cfg(not(target_os = "linux"))]
fn add_dir_watch(kq: i32, dir: &Path) -> Option<i32> {
    let Ok(c_dir) = CString::new(dir.as_os_str().as_bytes()) else {
        return None;
    };

    let fd = unsafe {
        libc::open(
            c_dir.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        trace!(
            "error opening directory {dir:?}: {}",
            io::Error::last_os_error()
        );
        return None;
    }

    let mut ev = unsafe { std::mem::zeroed::<libc::kevent>() };
    ev.ident = fd as usize;
    ev.filter = libc::EVFILT_VNODE;
    ev.flags = (libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR) as u16;
    ev.fflags = KQ_DIR_WATCH_FLAGS;

    if unsafe { libc::kevent(kq, &ev, 1, std::ptr::null_mut(), 0, std::ptr::null()) } < 0 {
        trace!(
            "error watching directory {dir:?}: {}",
            io::Error::last_os_error()
        );
        unsafe { libc::close(fd) };
        return None;
    }

    Some(fd)
}

/// Re-arms the kqueue directory watches for the include files.
///
/// Called on the watcher thread after every config parse, since the set of includes is only
/// known after parsing. Closing a stale fd automatically deregisters its kevent.
#[cfg(not(target_os = "linux"))]
fn update_include_watches(
    trigger_fd: i32,
    includes: &HashMap<PathBuf, Option<Props>>,
    watches: &mut HashMap<PathBuf, i32>,
) {
    let include_dirs: HashSet<PathBuf> = includes
        .keys()
        .filter_map(|path| path.parent())
        .filter_map(|dir| dir.canonicalize().ok())
        .collect();

    // Remove watches for directories that no longer contain include files.
    watches.retain(|dir, fd| {
        if include_dirs.contains(dir) {
            true
        } else {
            unsafe { libc::close(*fd) };
            false
        }
    });

    // Add watches for new include directories.
    for dir in include_dirs {
        if !watches.contains_key(&dir) {
            if let Some(fd) = add_dir_watch(trigger_fd, &dir) {
                watches.insert(dir, fd);
            }
        }
    }
}

/// Drains all pending kqueue events, calling `handle` for each one.
///
/// Returns the number of events handled.
#[cfg(not(target_os = "linux"))]
fn drain_trigger_events(kq: i32, mut handle: impl FnMut()) -> io::Result<usize> {
    let mut events = [unsafe { std::mem::zeroed::<libc::kevent>() }; 16];
    let mut handled = 0;
    loop {
        let n = unsafe {
            libc::kevent(
                kq,
                std::ptr::null(),
                0,
                events.as_mut_ptr(),
                events.len() as i32,
                std::ptr::null(),
            )
        };
        if n > 0 {
            for _event in &events[..n as usize] {
                handle();
                handled += 1;
            }
        } else if n == 0 {
            break;
        } else {
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(libc::EINTR) => continue,
                _ => return Err(err),
            }
        }
    }

    Ok(handled)
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs::{self, File, FileTimes};
    use std::io::Write;

    use xshell::{cmd, Shell, TempDir};

    use super::*;

    type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;

    fn canon(config_path: &ConfigPath) -> &PathBuf {
        match config_path {
            ConfigPath::Explicit(path) => path,
            ConfigPath::Regular {
                user_path,
                system_path,
            } => {
                if user_path.exists() {
                    user_path
                } else {
                    system_path
                }
            }
        }
    }

    enum TestPath<P> {
        Explicit(P),
        Regular { user_path: P, system_path: P },
    }

    impl<P: AsRef<Path>> TestPath<P> {
        fn setup<Discard>(
            self,
            setup: impl FnOnce(&Shell) -> xshell::Result<Discard>,
        ) -> TestSetup {
            self.setup_any(|sh| {
                _ = setup(sh)?;
                Ok(())
            })
        }

        fn without_setup(self) -> TestSetup {
            self.setup_any(|_| Ok(())).assert_initial_not_exists()
        }

        fn setup_any(self, setup: impl FnOnce(&Shell) -> Result) -> TestSetup {
            let sh = Shell::new().unwrap();
            let temp_dir = sh.create_temp_dir().unwrap();
            sh.change_dir(temp_dir.path());

            let dir = sh.current_dir();
            let config_path = match self {
                TestPath::Explicit(path) => ConfigPath::Explicit(dir.join(path)),
                TestPath::Regular {
                    user_path,
                    system_path,
                } => ConfigPath::Regular {
                    user_path: dir.join(user_path),
                    system_path: dir.join(system_path),
                },
            };

            setup(&sh).unwrap();

            TestSetup {
                sh,
                config_path,
                _temp_dir: temp_dir,
            }
        }
    }

    struct TestSetup {
        sh: Shell,
        config_path: ConfigPath,
        _temp_dir: TempDir,
    }

    impl TestSetup {
        fn assert_initial_not_exists(self) -> Self {
            let canon = canon(&self.config_path);
            assert!(!canon.exists(), "initial should not exist");
            self
        }

        fn assert_initial(self, expected: &str) -> Self {
            let canon = canon(&self.config_path);
            assert!(canon.exists(), "initial should exist at {canon:?}");
            let actual = fs::read_to_string(canon).unwrap();
            assert_eq!(actual, expected, "initial file contents do not match");
            self
        }

        fn run(self, body: impl FnOnce(&Shell, &mut TestUtil) -> Result) -> Result {
            let TestSetup {
                sh, config_path, ..
            } = self;

            let includes = config_path.load().includes;
            let mut test = TestUtil {
                watcher: WatcherInner::new(config_path, includes),
            };

            // don't trigger before we start
            test.assert_unchanged();
            // pass_time() inside assert_unchanged() ensures that mtime
            // isn't the same as the initial time

            body(&sh, &mut test)?;

            // nothing should trigger after the test runs
            test.assert_unchanged();

            Ok(())
        }
    }

    struct TestUtil {
        watcher: WatcherInner,
    }

    impl TestUtil {
        // Ensures that mtime is different between writes in the tests.
        fn pass_time(&self) {
            thread::sleep(Duration::from_millis(50));
        }

        fn assert_unchanged(&mut self) {
            let res = self.watcher.check();

            // This may be Missing or Unchanged, both are fine.
            assert_ne!(
                res,
                CheckResult::Changed,
                "watcher should not have noticed any changes"
            );

            self.pass_time();
        }

        fn assert_changed_to(&mut self, expected: &str) {
            let res = self.watcher.check();
            assert_eq!(
                res,
                CheckResult::Changed,
                "watcher should have noticed a change, but it didn't"
            );

            let new_path = canon(&self.watcher.path);
            let actual = fs::read_to_string(new_path).unwrap();
            assert_eq!(actual, expected, "wrong file contents");

            self.watcher.set_includes(Config::load(new_path).includes);

            self.pass_time();
        }
    }

    #[test]
    fn change_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                sh.write_file("niri/config.kdl", "b")?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn overwrite_but_dont_change_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                sh.write_file("niri/config.kdl", "a")?;
                test.assert_changed_to("a");

                Ok(())
            })
    }

    #[test]
    fn touch_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                cmd!(sh, "touch niri/config.kdl").run()?;
                test.assert_changed_to("a");

                Ok(())
            })
    }

    #[test]
    fn create_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.create_dir("niri"))
            .assert_initial_not_exists()
            .run(|sh, test| {
                sh.write_file("niri/config.kdl", "a")?;
                test.assert_changed_to("a");

                Ok(())
            })
    }

    #[test]
    fn create_dir_and_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .without_setup()
            .run(|sh, test| {
                sh.write_file("niri/config.kdl", "a")?;
                test.assert_changed_to("a");

                Ok(())
            })
    }

    #[test]
    fn change_linked_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri/config2.kdl", "a")?;
                cmd!(sh, "ln -sf config2.kdl niri/config.kdl").run()
            })
            .assert_initial("a")
            .run(|sh, test| {
                sh.write_file("niri/config2.kdl", "b")?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn change_file_in_linked_dir() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri2/config.kdl", "a")?;
                cmd!(sh, "ln -s niri2 niri").run()
            })
            .assert_initial("a")
            .run(|sh, test| {
                sh.write_file("niri2/config.kdl", "b")?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn remove_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                sh.remove_path("niri/config.kdl")?;
                test.assert_unchanged();

                Ok(())
            })
    }

    #[test]
    fn remove_dir() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                sh.remove_path("niri")?;
                test.assert_unchanged();

                Ok(())
            })
    }

    #[test]
    fn recreate_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                sh.remove_path("niri/config.kdl")?;
                sh.write_file("niri/config.kdl", "b")?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn recreate_dir() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri/config.kdl", "a")?;
                Ok(())
            })
            .assert_initial("a")
            .run(|sh, test| {
                sh.remove_path("niri")?;
                sh.write_file("niri/config.kdl", "b")?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn swap_dir() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| sh.write_file("niri/config.kdl", "a"))
            .assert_initial("a")
            .run(|sh, test| {
                sh.write_file("niri2/config.kdl", "b")?;
                sh.remove_path("niri")?;
                cmd!(sh, "mv niri2 niri").run()?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn swap_dir_link() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri2/config.kdl", "a")?;
                cmd!(sh, "ln -s niri2 niri").run()
            })
            .assert_initial("a")
            .run(|sh, test| {
                sh.write_file("niri3/config.kdl", "b")?;
                sh.remove_path("niri")?;
                cmd!(sh, "ln -s niri3 niri").run()?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn change_included_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri/config.kdl", "include \"colors.kdl\"")?;
                sh.write_file("niri/colors.kdl", "// Colors")
            })
            .assert_initial("include \"colors.kdl\"")
            .run(|sh, test| {
                sh.write_file("niri/colors.kdl", "// Updated colors")?;
                test.assert_changed_to("include \"colors.kdl\"");

                Ok(())
            })
    }

    #[test]
    fn remove_included_file() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri/config.kdl", "include \"colors.kdl\"")?;
                sh.write_file("niri/colors.kdl", "// Colors")
            })
            .assert_initial("include \"colors.kdl\"")
            .run(|sh, test| {
                sh.remove_path("niri/colors.kdl")?;
                test.assert_changed_to("include \"colors.kdl\"");

                Ok(())
            })
    }

    #[test]
    fn nested_includes() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri/config.kdl", "include \"a.kdl\"")?;
                sh.write_file("niri/a.kdl", "include \"b.kdl\"")?;
                sh.write_file("niri/b.kdl", "// b content")
            })
            .assert_initial("include \"a.kdl\"")
            .run(|sh, test| {
                sh.write_file("niri/b.kdl", "// updated b")?;
                test.assert_changed_to("include \"a.kdl\"");

                Ok(())
            })
    }

    #[test]
    fn broken_include_still_gets_watched() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup(|sh| {
                sh.write_file("niri/config.kdl", "include \"colors.kdl\"")?;
                sh.write_file("niri/colors.kdl", "broken")
            })
            .assert_initial("include \"colors.kdl\"")
            .run(|sh, test| {
                sh.write_file("niri/colors.kdl", "// Fixed")?;
                test.assert_changed_to("include \"colors.kdl\"");

                Ok(())
            })
    }

    // Important: On systems like NixOS, mtime is not kept for config files.
    // So, this is testing that the watcher handles that correctly.
    fn create_epoch(path: impl AsRef<Path>, content: &str) -> Result {
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;
        file.set_times(
            FileTimes::new()
                .set_accessed(SystemTime::UNIX_EPOCH)
                .set_modified(SystemTime::UNIX_EPOCH),
        )?;
        file.sync_all()?;
        Ok(())
    }

    #[test]
    fn swap_just_link() -> Result {
        TestPath::Explicit("niri/config.kdl")
            .setup_any(|sh| {
                let dir = sh.current_dir().join("niri");

                sh.create_dir(&dir)?;

                create_epoch(dir.join("config2.kdl"), "a")?;
                create_epoch(dir.join("config3.kdl"), "b")?;

                cmd!(sh, "ln -s config2.kdl niri/config.kdl").run()?;

                Ok(())
            })
            .assert_initial("a")
            .run(|sh, test| {
                cmd!(sh, "ln -sf config3.kdl niri/config.kdl").run()?;
                test.assert_changed_to("b");

                Ok(())
            })
    }

    #[test]
    fn swap_many_regular() -> Result {
        TestPath::Regular {
            user_path: "user-niri/config.kdl",
            system_path: "system-niri/config.kdl",
        }
        .setup(|sh| sh.write_file("system-niri/config.kdl", "system config"))
        .assert_initial("system config")
        .run(|sh, test| {
            sh.write_file("user-niri/config.kdl", "user config")?;
            test.assert_changed_to("user config");

            cmd!(sh, "touch system-niri/config.kdl").run()?;
            test.assert_unchanged();

            sh.remove_path("system-niri")?;
            test.assert_unchanged();

            sh.write_file("system-niri/config.kdl", "new system config")?;
            test.assert_unchanged();

            sh.remove_path("user-niri")?;
            test.assert_changed_to("new system config");

            sh.write_file("system-niri/config.kdl", "updated system config")?;
            test.assert_changed_to("updated system config");

            sh.write_file("user-niri/config.kdl", "new user config")?;
            test.assert_changed_to("new user config");

            Ok(())
        })
    }

    #[test]
    fn swap_many_links_regular_like_nix() -> Result {
        TestPath::Regular {
            user_path: "user-niri/config.kdl",
            system_path: "system-niri/config.kdl",
        }
        .setup_any(|sh| {
            let store = sh.current_dir().join("store");

            sh.create_dir(&store)?;

            create_epoch(store.join("gen1"), "gen 1")?;
            create_epoch(store.join("gen2"), "gen 2")?;
            create_epoch(store.join("gen3"), "gen 3")?;

            sh.create_dir("user-niri")?;
            sh.create_dir("system-niri")?;

            Ok(())
        })
        .assert_initial_not_exists()
        .run(|sh, test| {
            let store = sh.current_dir().join("store");
            test.assert_unchanged();

            cmd!(sh, "ln -s {store}/gen1 user-niri/config.kdl").run()?;
            test.assert_changed_to("gen 1");

            cmd!(sh, "ln -s {store}/gen2 system-niri/config.kdl").run()?;
            test.assert_unchanged();

            cmd!(sh, "unlink user-niri/config.kdl").run()?;
            test.assert_changed_to("gen 2");

            cmd!(sh, "ln -s {store}/gen3 user-niri/config.kdl").run()?;
            test.assert_changed_to("gen 3");

            cmd!(sh, "ln -sf {store}/gen1 system-niri/config.kdl").run()?;
            test.assert_unchanged();

            cmd!(sh, "unlink system-niri/config.kdl").run()?;
            test.assert_unchanged();

            cmd!(sh, "ln -s {store}/gen1 system-niri/config.kdl").run()?;
            test.assert_unchanged();

            cmd!(sh, "unlink user-niri/config.kdl").run()?;
            test.assert_changed_to("gen 1");

            Ok(())
        })
    }

    // Drives the inotify drain loop through a real inotify fd, verifying the offset math that
    // advances past each event's name, the EAGAIN termination, and the IN_IGNORED filtering.
    #[test]
    #[cfg(target_os = "linux")]
    fn drain_trigger_events_real_fd() -> Result {
        let sh = Shell::new()?;
        let temp_dir = sh.create_temp_dir()?;
        let dir = temp_dir.path();

        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        assert!(
            fd >= 0,
            "inotify_init1 failed: {}",
            io::Error::last_os_error()
        );

        let c_dir = CString::new(dir.as_os_str().as_bytes()).unwrap();
        let wd = unsafe { libc::inotify_add_watch(fd, c_dir.as_ptr(), DIR_WATCH_MASK) };
        assert!(
            wd >= 0,
            "inotify_add_watch failed: {}",
            io::Error::last_os_error()
        );

        // Write several files with names of different lengths, so that the drain loop has to
        // advance past names of varying sizes within one buffer.
        let names = ["a.kdl", "config-very-long-name.kdl", "zzz.kdl"];
        for name in &names {
            fs::write(dir.join(name), "x")?;
        }

        // Count the events we see.
        let mut count = 0usize;
        let handled = drain_trigger_events(fd, || {
            count += 1;
        })?;

        // Each file should have produced at least one event.
        assert!(
            handled >= names.len(),
            "expected at least {} events, got {handled}",
            names.len()
        );
        assert_eq!(count, handled);

        // Removing the watch queues an IN_IGNORED event, which the drain loop must skip rather
        // than treat as a config change.
        unsafe { libc::inotify_rm_watch(fd, wd) };
        let handled = drain_trigger_events(fd, || {})?;
        assert_eq!(handled, 0);

        unsafe { libc::close(fd) };
        Ok(())
    }
}
