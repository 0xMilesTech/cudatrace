use crate::reentry::HookBypassGuard;
use std::ffi::c_void;

pub unsafe fn resolve_next_from_ptr<T: Copy>(symbol: *const crate::ffi::c_char) -> Option<T> {
    let _bypass = HookBypassGuard::enter();
    // SAFETY: symbol must be a valid NUL-terminated string and T must match symbol signature.
    let ptr = unsafe { crate::ffi::dlsym(crate::ffi::RTLD_NEXT, symbol) };
    if ptr.is_null() {
        None
    } else {
        // SAFETY: caller guarantees function signature compatibility.
        Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&ptr) })
    }
}
