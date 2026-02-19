use crate::config::{TimeUnit, global};
use std::cell::Cell;
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static CACHED_TID: Cell<i32> = const { Cell::new(0) };
}

pub fn current_tid() -> i32 {
    CACHED_TID.with(|cached| {
        let tid = cached.get();
        if tid > 0 {
            return tid;
        }
        // SAFETY: syscall with SYS_GETTID has no memory safety implications.
        let tid = unsafe { crate::ffi::syscall(crate::ffi::SYS_GETTID) as i32 };
        if tid > 0 {
            cached.set(tid);
        }
        tid
    })
}

pub fn now_timestamp_for_config() -> u128 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    match global().time_unit {
        TimeUnit::Ns => duration.as_nanos(),
        TimeUnit::Us => duration.as_micros(),
    }
}

pub fn prepend_left_meta(line: &str, tid: i32, timestamp: u128) -> String {
    let prefix = format_left_meta_prefix(tid, timestamp);
    if prefix.is_empty() {
        line.to_owned()
    } else {
        format!("{prefix}{line}")
    }
}

fn format_left_meta_prefix(tid: i32, timestamp: u128) -> String {
    let cfg = global();
    let mut out = String::new();

    if cfg.left_meta.include_tid() {
        let _ = write!(&mut out, "tid={tid}");
    }
    if cfg.left_meta.include_timestamp() {
        if !out.is_empty() {
            out.push(' ');
        }
        let unit = match cfg.time_unit {
            TimeUnit::Ns => "ns",
            TimeUnit::Us => "us",
        };
        let _ = write!(&mut out, "ts={timestamp}{unit}");
    }

    if out.is_empty() {
        String::new()
    } else {
        out.push_str("  |  ");
        out
    }
}
