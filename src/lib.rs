#![allow(non_camel_case_types)]

mod config;
mod decode;
mod dlsym;
mod fd_map;
mod ffi;
mod graph;
mod hook;
mod line_meta;
mod logger;
mod reentry;

pub use config::{
    Config, IoctlDecodeMode, LeftMetaMode, OutputMode, TimeUnit, TraceDomain, TraceMask,
    global as global_config,
};

#[used]
#[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
static CUDATRACE_INIT: [unsafe extern "C" fn(); 1] = [cudatrace_init];

unsafe extern "C" fn cudatrace_init() {
    hook::ptrace::ensure_started();
}

#[unsafe(no_mangle)]
pub extern "C" fn cudatrace_version() -> *const crate::ffi::c_char {
    c"cudatrace 0.1.0".as_ptr()
}
