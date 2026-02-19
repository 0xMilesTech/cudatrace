use crate::config::{OutputMode, global};
use crate::reentry::HookBypassGuard;
use std::ffi::CString;
use std::os::fd::RawFd;
use std::sync::{LazyLock, Mutex};

thread_local! {
    static THREAD_FILE_SINK: std::cell::RefCell<Option<ThreadFileSink>> = const { std::cell::RefCell::new(None) };
}

static LOGGER_SETTINGS: LazyLock<LoggerSettings> = LazyLock::new(LoggerSettings::from_config);
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

enum LoggerTarget {
    Stdout,
    ThreadFiles { base_path: String },
}

struct LoggerSettings {
    target: LoggerTarget,
}

struct ThreadFileSink {
    fd: RawFd,
}

impl LoggerSettings {
    fn from_config() -> Self {
        let cfg = global();
        match cfg.output {
            OutputMode::Stdout => Self {
                target: LoggerTarget::Stdout,
            },
            OutputMode::File => Self {
                target: LoggerTarget::ThreadFiles {
                    base_path: cfg.path.clone(),
                },
            },
        }
    }
}

impl Drop for ThreadFileSink {
    fn drop(&mut self) {
        let _bypass = HookBypassGuard::enter();
        // SAFETY: fd is owned by this sink.
        unsafe {
            let _ = crate::ffi::syscall(crate::ffi::SYS_CLOSE, self.fd);
        }
    }
}

pub fn log_line(line: &str) {
    let _bypass = HookBypassGuard::enter();

    let mut bytes = Vec::with_capacity(line.len() + 1);
    bytes.extend_from_slice(line.as_bytes());
    bytes.push(b'\n');

    match &LOGGER_SETTINGS.target {
        LoggerTarget::Stdout => {
            let _lock = STDOUT_LOCK.lock().expect("stdout lock poisoned");
            let _ = write_all(crate::ffi::STDOUT_FILENO, &bytes);
        }
        LoggerTarget::ThreadFiles { base_path } => {
            let write_result = match THREAD_FILE_SINK.try_with(|sink_cell| {
                let needs_init = sink_cell.borrow().is_none();
                if needs_init {
                    let tid = current_tid();
                    let thread_path = thread_output_path(base_path, tid);
                    match open_output_file(&thread_path) {
                        Some(fd) => {
                            sink_cell.borrow_mut().replace(ThreadFileSink { fd });
                        }
                        None => {
                            let _ = write_all(
                                crate::ffi::STDERR_FILENO,
                                b"[cudatrace] failed to open thread output file, fallback to stdout\n",
                            );
                            return write_all(crate::ffi::STDOUT_FILENO, &bytes);
                        }
                    }
                }

                let fd = sink_cell
                    .borrow()
                    .as_ref()
                    .map(|sink| sink.fd)
                    .unwrap_or(crate::ffi::STDOUT_FILENO);
                write_all(fd, &bytes)
            }) {
                Ok(result) => result,
                Err(_) => write_all(crate::ffi::STDOUT_FILENO, &bytes),
            };

            if write_result.is_err() {
                let _lock = STDOUT_LOCK.lock().expect("stdout lock poisoned");
                let _ = write_all(crate::ffi::STDOUT_FILENO, &bytes);
            }
        }
    }
}

fn open_output_file(path: &str) -> Option<RawFd> {
    let path_c = CString::new(path.as_bytes()).ok()?;

    // SAFETY: path_c is a valid C string and syscall arguments are valid.
    let fd = unsafe {
        crate::ffi::syscall(
            crate::ffi::SYS_OPENAT,
            crate::ffi::AT_FDCWD,
            path_c.as_ptr(),
            crate::ffi::O_CREAT
                | crate::ffi::O_WRONLY
                | crate::ffi::O_APPEND
                | crate::ffi::O_CLOEXEC,
            0o644,
        ) as crate::ffi::c_int
    };

    if fd >= 0 { Some(fd) } else { None }
}

fn write_all(fd: RawFd, mut data: &[u8]) -> Result<(), ()> {
    while !data.is_empty() {
        // SAFETY: data points to initialized memory and fd is expected to be valid.
        let written = unsafe {
            crate::ffi::write(
                fd,
                data.as_ptr().cast::<crate::ffi::c_void>(),
                data.len() as crate::ffi::size_t,
            )
        };

        if written < 0 {
            // SAFETY: errno pointer is provided by libc.
            let err = unsafe { *crate::ffi::__errno_location() };
            if err == crate::ffi::EINTR {
                continue;
            }
            return Err(());
        }

        let written = written as usize;
        data = &data[written..];
    }

    Ok(())
}

fn current_tid() -> i64 {
    // SAFETY: syscall with SYS_GETTID has no memory safety implications.
    unsafe { crate::ffi::syscall(crate::ffi::SYS_GETTID) as i64 }
}

fn thread_output_path(base_path: &str, tid: i64) -> String {
    format!("{base_path}.tid-{tid}")
}

#[cfg(test)]
mod tests {
    use super::thread_output_path;

    #[test]
    fn thread_output_path_appends_tid_suffix() {
        let path = thread_output_path("./cudatrace.output", 1234);
        assert_eq!(path, "./cudatrace.output.tid-1234");
    }
}
