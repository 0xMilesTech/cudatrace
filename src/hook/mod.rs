pub mod cudart;
pub mod driver;
pub mod ptrace;

use crate::config::{TraceDomain, global};
use crate::graph::{SpanToken, enter, exit};
use crate::reentry::{HookBypassGuard, hooks_blocked};

pub struct TraceScope {
    span: Option<SpanToken>,
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
            return Self { span: None };
        }
        crate::hook::ptrace::ensure_started();
        if !global().trace_enabled(domain) || hooks_blocked() {
            return Self { span: None };
        }

        let _bypass = HookBypassGuard::enter();
        let span = enter(func, build_args());
        Self { span: Some(span) }
    }

    pub fn finish(self, ret: String) {
        if let Some(span) = self.span {
            let _bypass = HookBypassGuard::enter();
            exit(span, ret);
        }
    }
}
