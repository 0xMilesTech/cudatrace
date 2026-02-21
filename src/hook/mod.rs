pub mod cudart;
pub mod driver;
pub mod fgraph;
pub mod ptrace;

use crate::config::{TraceDomain, global};
use crate::graph::{SpanToken, enter, exit};
use crate::reentry::{HookBypassGuard, hooks_blocked};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

thread_local! {
    static FGRAPH_WINDOW_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn next_fgraph_window_seq(func: &'static str) -> u32 {
    static WINDOW_SEQ: OnceLock<Mutex<HashMap<&'static str, u32>>> = OnceLock::new();
    let map = WINDOW_SEQ.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let next = guard.get(func).copied().unwrap_or(0).saturating_add(1);
    guard.insert(func, next);
    next
}

pub struct TraceScope {
    span: Option<SpanToken>,
    fgraph_window_matched: bool,
}

fn enter_fgraph_window_if_matched(func: &'static str) -> bool {
    let cfg = global();
    if !cfg.fgraph_enabled() || !cfg.fgraph_match(func) {
        return false;
    }

    let seq = next_fgraph_window_seq(func);
    let depth = FGRAPH_WINDOW_DEPTH.with(|value| {
        let next = value.get().saturating_add(1);
        value.set(next);
        next
    });
    crate::hook::ptrace::publish_fgraph_window_enter(depth, func, seq);
    true
}

fn exit_fgraph_window_if_needed(matched: bool) {
    if !matched {
        return;
    }

    let depth = FGRAPH_WINDOW_DEPTH.with(|value| {
        let next = value.get().saturating_sub(1);
        value.set(next);
        next
    });
    crate::hook::ptrace::publish_fgraph_window_exit(depth);
}

impl TraceScope {
    pub fn enter(domain: TraceDomain, func: &'static str, args: String) -> Self {
        Self::enter_lazy(domain, func, || args)
    }

    pub fn enter_lazy<F>(domain: TraceDomain, func: &'static str, build_args: F) -> Self
    where
        F: FnOnce() -> String,
    {
        if crate::hook::ptrace::is_internal_tracer() {
            return Self {
                span: None,
                fgraph_window_matched: false,
            };
        }
        crate::hook::ptrace::ensure_started();
        let blocked = hooks_blocked();
        let fgraph_window_matched = if blocked {
            false
        } else {
            enter_fgraph_window_if_matched(func)
        };

        if !global().trace_enabled(domain) || blocked {
            return Self {
                span: None,
                fgraph_window_matched,
            };
        }

        let _bypass = HookBypassGuard::enter();
        let span = enter(func, build_args());
        Self {
            span: Some(span),
            fgraph_window_matched,
        }
    }

    pub fn finish(self, ret: String) {
        exit_fgraph_window_if_needed(self.fgraph_window_matched);
        if let Some(span) = self.span {
            let _bypass = HookBypassGuard::enter();
            exit(span, ret);
        }
    }
}
