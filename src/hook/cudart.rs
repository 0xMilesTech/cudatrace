use crate::config::TraceDomain;
use crate::dlsym::resolve_next_from_ptr;
use crate::hook::TraceScope;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::sync::{Mutex, OnceLock};

pub type cudaError_t = crate::ffi::c_int;
pub type cudaStream_t = *mut c_void;
pub type cudaEvent_t = *mut c_void;
pub type cudaMemcpyKind = crate::ffi::c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct dim3 {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

const CUDA_ERROR_UNKNOWN: cudaError_t = 999;

#[cfg(has_generated_cudart_wrappers)]
unsafe extern "C" {
    fn cudatrace_lookup_generated_cudart_wrapper(symbol: *const c_char) -> *mut c_void;
}

macro_rules! resolve_next {
    ($cell:ident, $name:literal, $ty:ty) => {{
        static $cell: OnceLock<Option<$ty>> = OnceLock::new();
        *$cell.get_or_init(|| unsafe {
            resolve_next_from_ptr::<$ty>(concat!($name, "\0").as_ptr().cast::<crate::ffi::c_char>())
        })
    }};
}

fn intern_cudart_func_name(symbol: &CStr) -> &'static str {
    static INTERN: OnceLock<Mutex<HashMap<Vec<u8>, &'static str>>> = OnceLock::new();
    let key = symbol.to_bytes().to_vec();
    let map = INTERN.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(&name) = guard.get(&key) {
        return name;
    }

    let leaked: &'static str =
        Box::leak(String::from_utf8_lossy(&key).into_owned().into_boxed_str());
    guard.insert(key, leaked);
    leaked
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudatrace_cudart_enter(func: *const c_char) -> *mut c_void {
    if func.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: generated wrappers pass valid static symbol names.
    let func = unsafe { CStr::from_ptr(func) };
    let func = intern_cudart_func_name(func);
    let scope = Box::new(TraceScope::enter(TraceDomain::Cudart, func, String::new()));
    Box::into_raw(scope).cast::<c_void>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudatrace_cudart_exit(scope: *mut c_void, ret: cudaError_t) {
    if scope.is_null() {
        return;
    }

    // SAFETY: pointer originates from cudatrace_cudart_enter and is consumed once.
    let scope: Box<TraceScope> = unsafe { Box::from_raw(scope.cast::<TraceScope>()) };
    scope.finish(ret.to_string());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudatrace_resolve_cudart_symbol(symbol: *const c_char) -> *mut c_void {
    if symbol.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: caller passes a dlsym-compatible symbol pointer.
    unsafe { resolve_next_from_ptr::<*mut c_void>(symbol) }.unwrap_or(std::ptr::null_mut())
}

#[cfg(has_generated_cudart_wrappers)]
fn lookup_generated_cudart_wrapper(symbol: *const c_char) -> *mut c_void {
    // SAFETY: symbol pointer is valid for lookup call.
    unsafe { cudatrace_lookup_generated_cudart_wrapper(symbol) }
}

#[cfg(not(has_generated_cudart_wrappers))]
fn lookup_generated_cudart_wrapper(_symbol: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

pub(crate) fn wrapper_for_cudart_symbol(symbol: &CStr) -> Option<*mut c_void> {
    let ptr = match symbol.to_bytes() {
        b"cudaMalloc" => cudaMalloc as *const () as *mut c_void,
        b"cudaFree" => cudaFree as *const () as *mut c_void,
        b"cudaHostAlloc" => cudaHostAlloc as *const () as *mut c_void,
        b"cudaMallocHost" => cudaMallocHost as *const () as *mut c_void,
        b"cudaFreeHost" => cudaFreeHost as *const () as *mut c_void,
        b"cudaMemcpyAsync" => cudaMemcpyAsync as *const () as *mut c_void,
        b"cudaLaunchKernel" => cudaLaunchKernel as *const () as *mut c_void,
        b"cudaStreamCreate" => cudaStreamCreate as *const () as *mut c_void,
        b"cudaStreamDestroy" => cudaStreamDestroy as *const () as *mut c_void,
        b"cudaEventCreate" => cudaEventCreate as *const () as *mut c_void,
        b"cudaEventRecord" => cudaEventRecord as *const () as *mut c_void,
        b"cudaEventSynchronize" => cudaEventSynchronize as *const () as *mut c_void,
        b"cudaEventElapsedTime" => cudaEventElapsedTime as *const () as *mut c_void,
        b"cudaDeviceReset" => cudaDeviceReset as *const () as *mut c_void,
        _ => std::ptr::null_mut(),
    };

    if !ptr.is_null() {
        return Some(ptr);
    }

    let generated = lookup_generated_cudart_wrapper(symbol.as_ptr());
    if !generated.is_null() {
        return Some(generated);
    }

    None
}

type CudaMallocFn = unsafe extern "C" fn(*mut *mut c_void, usize) -> cudaError_t;
type CudaFreeFn = unsafe extern "C" fn(*mut c_void) -> cudaError_t;
type CudaHostAllocFn =
    unsafe extern "C" fn(*mut *mut c_void, usize, crate::ffi::c_uint) -> cudaError_t;
type CudaMallocHostFn = unsafe extern "C" fn(*mut *mut c_void, usize) -> cudaError_t;
type CudaFreeHostFn = unsafe extern "C" fn(*mut c_void) -> cudaError_t;
type CudaMemcpyAsyncFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    cudaMemcpyKind,
    cudaStream_t,
) -> cudaError_t;
type CudaLaunchKernelFn = unsafe extern "C" fn(
    *const c_void,
    dim3,
    dim3,
    *mut *mut c_void,
    usize,
    cudaStream_t,
) -> cudaError_t;
type CudaStreamCreateFn = unsafe extern "C" fn(*mut cudaStream_t) -> cudaError_t;
type CudaStreamDestroyFn = unsafe extern "C" fn(cudaStream_t) -> cudaError_t;
type CudaEventCreateFn = unsafe extern "C" fn(*mut cudaEvent_t) -> cudaError_t;
type CudaEventRecordFn = unsafe extern "C" fn(cudaEvent_t, cudaStream_t) -> cudaError_t;
type CudaEventSynchronizeFn = unsafe extern "C" fn(cudaEvent_t) -> cudaError_t;
type CudaEventElapsedTimeFn =
    unsafe extern "C" fn(*mut f32, cudaEvent_t, cudaEvent_t) -> cudaError_t;
type CudaDeviceResetFn = unsafe extern "C" fn() -> cudaError_t;

fn real_cuda_malloc() -> Option<CudaMallocFn> {
    resolve_next!(REAL_CUDA_MALLOC, "cudaMalloc", CudaMallocFn)
}

fn real_cuda_free() -> Option<CudaFreeFn> {
    resolve_next!(REAL_CUDA_FREE, "cudaFree", CudaFreeFn)
}

fn real_cuda_host_alloc() -> Option<CudaHostAllocFn> {
    resolve_next!(REAL_CUDA_HOST_ALLOC, "cudaHostAlloc", CudaHostAllocFn)
}

fn real_cuda_malloc_host() -> Option<CudaMallocHostFn> {
    resolve_next!(REAL_CUDA_MALLOC_HOST, "cudaMallocHost", CudaMallocHostFn)
}

fn real_cuda_free_host() -> Option<CudaFreeHostFn> {
    resolve_next!(REAL_CUDA_FREE_HOST, "cudaFreeHost", CudaFreeHostFn)
}

fn real_cuda_memcpy_async() -> Option<CudaMemcpyAsyncFn> {
    resolve_next!(REAL_CUDA_MEMCPY_ASYNC, "cudaMemcpyAsync", CudaMemcpyAsyncFn)
}

fn real_cuda_launch_kernel() -> Option<CudaLaunchKernelFn> {
    resolve_next!(
        REAL_CUDA_LAUNCH_KERNEL,
        "cudaLaunchKernel",
        CudaLaunchKernelFn
    )
}

fn real_cuda_stream_create() -> Option<CudaStreamCreateFn> {
    resolve_next!(
        REAL_CUDA_STREAM_CREATE,
        "cudaStreamCreate",
        CudaStreamCreateFn
    )
}

fn real_cuda_stream_destroy() -> Option<CudaStreamDestroyFn> {
    resolve_next!(
        REAL_CUDA_STREAM_DESTROY,
        "cudaStreamDestroy",
        CudaStreamDestroyFn
    )
}

fn real_cuda_event_create() -> Option<CudaEventCreateFn> {
    resolve_next!(REAL_CUDA_EVENT_CREATE, "cudaEventCreate", CudaEventCreateFn)
}

fn real_cuda_event_record() -> Option<CudaEventRecordFn> {
    resolve_next!(REAL_CUDA_EVENT_RECORD, "cudaEventRecord", CudaEventRecordFn)
}

fn real_cuda_event_synchronize() -> Option<CudaEventSynchronizeFn> {
    resolve_next!(
        REAL_CUDA_EVENT_SYNCHRONIZE,
        "cudaEventSynchronize",
        CudaEventSynchronizeFn
    )
}

fn real_cuda_event_elapsed_time() -> Option<CudaEventElapsedTimeFn> {
    resolve_next!(
        REAL_CUDA_EVENT_ELAPSED_TIME,
        "cudaEventElapsedTime",
        CudaEventElapsedTimeFn
    )
}

fn real_cuda_device_reset() -> Option<CudaDeviceResetFn> {
    resolve_next!(REAL_CUDA_DEVICE_RESET, "cudaDeviceReset", CudaDeviceResetFn)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaMalloc(dev_ptr: *mut *mut c_void, size: usize) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaMalloc",
        format!("dev_ptr={dev_ptr:p}, size={size}"),
    );

    let ret = if let Some(real) = real_cuda_malloc() {
        unsafe { real(dev_ptr, size) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaFree(dev_ptr: *mut c_void) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaFree",
        format!("dev_ptr={dev_ptr:p}"),
    );

    let ret = if let Some(real) = real_cuda_free() {
        unsafe { real(dev_ptr) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaHostAlloc(
    ptr: *mut *mut c_void,
    size: usize,
    flags: crate::ffi::c_uint,
) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaHostAlloc",
        format!("ptr={ptr:p}, size={size}, flags=0x{flags:x}"),
    );

    let ret = if let Some(real) = real_cuda_host_alloc() {
        unsafe { real(ptr, size, flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaMallocHost(ptr: *mut *mut c_void, size: usize) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaMallocHost",
        format!("ptr={ptr:p}, size={size}"),
    );

    let ret = if let Some(real) = real_cuda_malloc_host() {
        unsafe { real(ptr, size) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaFreeHost(ptr: *mut c_void) -> cudaError_t {
    let scope = TraceScope::enter(TraceDomain::Cudart, "cudaFreeHost", format!("ptr={ptr:p}"));

    let ret = if let Some(real) = real_cuda_free_host() {
        unsafe { real(ptr) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaMemcpyAsync(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
    kind: cudaMemcpyKind,
    stream: cudaStream_t,
) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaMemcpyAsync",
        format!("dst={dst:p}, src={src:p}, count={count}, kind={kind}, stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cuda_memcpy_async() {
        unsafe { real(dst, src, count, kind, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaLaunchKernel(
    func: *const c_void,
    grid_dim: dim3,
    block_dim: dim3,
    args: *mut *mut c_void,
    shared_mem: usize,
    stream: cudaStream_t,
) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaLaunchKernel",
        format!(
            "func={func:p}, grid=({}, {}, {}), block=({}, {}, {}), args={args:p}, sharedMem={}, stream={stream:p}",
            grid_dim.x, grid_dim.y, grid_dim.z, block_dim.x, block_dim.y, block_dim.z, shared_mem
        ),
    );

    let ret = if let Some(real) = real_cuda_launch_kernel() {
        unsafe { real(func, grid_dim, block_dim, args, shared_mem, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaStreamCreate(stream: *mut cudaStream_t) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaStreamCreate",
        format!("stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cuda_stream_create() {
        unsafe { real(stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaStreamDestroy(stream: cudaStream_t) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaStreamDestroy",
        format!("stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cuda_stream_destroy() {
        unsafe { real(stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaEventCreate(event: *mut cudaEvent_t) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaEventCreate",
        format!("event={event:p}"),
    );

    let ret = if let Some(real) = real_cuda_event_create() {
        unsafe { real(event) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaEventRecord(event: cudaEvent_t, stream: cudaStream_t) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaEventRecord",
        format!("event={event:p}, stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cuda_event_record() {
        unsafe { real(event, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaEventSynchronize(event: cudaEvent_t) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaEventSynchronize",
        format!("event={event:p}"),
    );

    let ret = if let Some(real) = real_cuda_event_synchronize() {
        unsafe { real(event) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaEventElapsedTime(
    ms: *mut f32,
    start: cudaEvent_t,
    end: cudaEvent_t,
) -> cudaError_t {
    let scope = TraceScope::enter(
        TraceDomain::Cudart,
        "cudaEventElapsedTime",
        format!("ms={ms:p}, start={start:p}, end={end:p}"),
    );

    let ret = if let Some(real) = real_cuda_event_elapsed_time() {
        unsafe { real(ms, start, end) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudaDeviceReset() -> cudaError_t {
    let scope = TraceScope::enter(TraceDomain::Cudart, "cudaDeviceReset", String::new());

    let ret = if let Some(real) = real_cuda_device_reset() {
        unsafe { real() }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[cfg(test)]
mod tests {
    use super::wrapper_for_cudart_symbol;

    #[test]
    fn wrapper_table_covers_core_runtime_symbols() {
        assert!(wrapper_for_cudart_symbol(c"cudaMalloc").is_some());
        assert!(wrapper_for_cudart_symbol(c"cudaMemcpyAsync").is_some());
        assert!(wrapper_for_cudart_symbol(c"cudaUnknown").is_none());
    }
}
