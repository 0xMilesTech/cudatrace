use crate::config::{TimeUnit, global};
use crate::line_meta::{current_tid, now_timestamp_for_config, prepend_left_meta};
use crate::logger::log_line;
use std::cell::RefCell;
use std::time::{Duration, Instant};

thread_local! {
    static STACK: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug)]
struct Frame {
    func: &'static str,
    args: String,
    depth: usize,
    started_ts: u128,
    started_at: Instant,
    has_child: bool,
    enter_printed: bool,
}

#[derive(Debug)]
pub struct SpanToken;

fn force_brace_mode() -> bool {
    let cfg = global();
    cfg.trace.syscall
}

pub fn enter(func: &'static str, args: String) -> SpanToken {
    let now = Instant::now();
    let now_ts = if global().left_meta.include_timestamp() {
        now_timestamp_for_config()
    } else {
        0
    };
    let mut parent_to_open: Option<(usize, &'static str, String, u128)> = None;
    let mut self_to_open: Option<(usize, &'static str, String, u128)> = None;
    let mut current_depth = 0_usize;
    let eager_open = force_brace_mode();

    let pushed = STACK
        .try_with(|stack| {
            let mut stack = stack.borrow_mut();
            if let Some(parent) = stack.last_mut() {
                if !parent.has_child {
                    parent.has_child = true;
                    if !parent.enter_printed {
                        parent.enter_printed = true;
                        parent_to_open = Some((
                            parent.depth,
                            parent.func,
                            parent.args.clone(),
                            parent.started_ts,
                        ));
                    }
                }
            }

            let depth = stack.len();
            if eager_open {
                self_to_open = Some((depth, func, args.clone(), now_ts));
            }
            stack.push(Frame {
                func,
                args,
                depth,
                started_ts: now_ts,
                started_at: now,
                has_child: false,
                enter_printed: eager_open,
            });
            current_depth = stack.len();
        })
        .is_ok();

    if !pushed {
        return SpanToken;
    }
    crate::hook::ptrace::publish_graph_depth(current_depth);
    let tid = current_tid();

    if let Some((depth, func, args, started_ts)) = parent_to_open {
        log_line(&with_left_meta(
            &format_open_line(depth, func, &args),
            tid,
            started_ts,
        ));
    }
    if let Some((depth, func, args, started_ts)) = self_to_open {
        log_line(&with_left_meta(
            &format_open_line(depth, func, &args),
            tid,
            started_ts,
        ));
    }

    SpanToken
}

pub fn exit(_token: SpanToken, ret: String) {
    let (frame, current_depth) = STACK
        .try_with(|stack| {
            let mut stack = stack.borrow_mut();
            let frame = stack.pop();
            let depth = stack.len();
            (frame, depth)
        })
        .ok()
        .unwrap_or((None, 0));

    let Some(frame) = frame else {
        return;
    };
    crate::hook::ptrace::publish_graph_depth(current_depth);
    let tid = current_tid();

    let elapsed = frame.started_at.elapsed();
    let (elapsed_value, elapsed_unit) = duration_for_config(elapsed);
    if force_brace_mode() || frame.has_child {
        if !frame.enter_printed {
            log_line(&with_left_meta(
                &format_open_line(frame.depth, frame.func, &frame.args),
                tid,
                frame.started_ts,
            ));
        }
        log_line(&with_left_meta(
            &format_close_line(frame.depth, &ret, elapsed_value, elapsed_unit),
            tid,
            frame.started_ts,
        ));
    } else {
        log_line(&with_left_meta(
            &format_leaf_line(
                frame.depth,
                frame.func,
                &frame.args,
                &ret,
                elapsed_value,
                elapsed_unit,
            ),
            tid,
            frame.started_ts,
        ));
    }
}

fn duration_for_config(duration: Duration) -> (u128, &'static str) {
    match global().time_unit {
        TimeUnit::Ns => (duration.as_nanos(), "ns"),
        TimeUnit::Us => (duration.as_micros(), "us"),
    }
}

fn format_open_line(depth: usize, func: &'static str, args: &str) -> String {
    let indent = "\t".repeat(depth);
    format!("{indent}{func}({args}) {{")
}

fn format_close_line(depth: usize, ret: &str, elapsed_value: u128, elapsed_unit: &str) -> String {
    let indent = "\t".repeat(depth);
    format!("{indent}}} = {ret}  /* {elapsed_value} {elapsed_unit} */")
}

fn format_leaf_line(
    depth: usize,
    func: &'static str,
    args: &str,
    ret: &str,
    elapsed_value: u128,
    elapsed_unit: &str,
) -> String {
    let indent = "\t".repeat(depth);
    format!("{indent}{func}({args}) = {ret}  /* {elapsed_value} {elapsed_unit} */")
}

fn with_left_meta(line: &str, tid: i32, timestamp: u128) -> String {
    prepend_left_meta(line, tid, timestamp)
}

#[cfg(test)]
mod tests {
    use super::{format_close_line, format_leaf_line, format_open_line};

    #[test]
    fn leaf_line_has_no_braces() {
        let line = format_leaf_line(1, "foo", "x=1", "0", 7, "us");
        assert!(!line.contains("{"));
        assert!(!line.contains("} ="));
        assert!(line.contains("foo(x=1) = 0"));
    }

    #[test]
    fn open_and_close_lines_keep_braces() {
        let open = format_open_line(0, "foo", "");
        let close = format_close_line(0, "0", 9, "us");
        assert!(open.ends_with(" {"));
        assert!(close.contains("} = 0"));
    }

    #[test]
    fn golden_trace_line_format_stability() {
        let open = format_open_line(1, "cudaHostAlloc", "ptr=0x1, size=64, flags=0x0");
        let leaf = format_leaf_line(1, "close", "fd=7", "0", 5, "us");
        let close = format_close_line(1, "304", 1572, "us");

        assert_eq!(open, "\tcudaHostAlloc(ptr=0x1, size=64, flags=0x0) {");
        assert_eq!(leaf, "\tclose(fd=7) = 0  /* 5 us */");
        assert_eq!(close, "\t} = 304  /* 1572 us */");
    }
}
