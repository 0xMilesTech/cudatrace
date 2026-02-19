use std::cell::Cell;

thread_local! {
    static BYPASS_DEPTH: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug)]
pub struct HookBypassGuard {
    active: bool,
}

impl HookBypassGuard {
    pub fn enter() -> Self {
        let active = BYPASS_DEPTH
            .try_with(|depth| {
                let current = depth.get();
                depth.set(current.saturating_add(1));
            })
            .is_ok();
        Self { active }
    }
}

pub fn hooks_blocked() -> bool {
    // If TLS is already tearing down, disable hooks to avoid panics.
    BYPASS_DEPTH
        .try_with(|depth| depth.get() > 0)
        .unwrap_or(true)
}

impl Drop for HookBypassGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let _ = BYPASS_DEPTH.try_with(|depth| {
            let current = depth.get();
            depth.set(current.saturating_sub(1));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{HookBypassGuard, hooks_blocked};

    #[test]
    fn bypass_blocks_hooks_in_scope() {
        assert!(!hooks_blocked());
        let _guard = HookBypassGuard::enter();
        assert!(hooks_blocked());
    }

    #[test]
    fn nested_bypass_scope_restores_state() {
        {
            let _outer = HookBypassGuard::enter();
            assert!(hooks_blocked());
            {
                let _inner = HookBypassGuard::enter();
                assert!(hooks_blocked());
            }
            assert!(hooks_blocked());
        }
        assert!(!hooks_blocked());
    }
}
