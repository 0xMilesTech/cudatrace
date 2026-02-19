use crate::config::TraceDomain;
use crate::dlsym::resolve_next_from_ptr;
use crate::hook::TraceScope;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

pub type CUresult = crate::ffi::c_int;
pub type CUdevice = crate::ffi::c_int;
pub type CUcontext = *mut c_void;
pub type CUmodule = *mut c_void;
pub type CUfunction = *mut c_void;
pub type CUstream = *mut c_void;
pub type CUevent = *mut c_void;
pub type CUdeviceptr = u64;
pub type cuuint64_t = u64;
pub type CUdriverProcAddressQueryResult = crate::ffi::c_int;

const CUDA_ERROR_UNKNOWN: CUresult = 999;
const CUDA_SUCCESS: CUresult = 0;
static CUDA_DRIVER_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

#[cfg(has_generated_driver_wrappers)]
unsafe extern "C" {
    fn cudatrace_lookup_generated_driver_wrapper(symbol: *const c_char) -> *mut c_void;
}

macro_rules! resolve_next {
    ($cell:ident, $name:literal, $ty:ty) => {{
        static $cell: OnceLock<Option<$ty>> = OnceLock::new();
        *$cell.get_or_init(|| unsafe {
            resolve_driver_from_ptr::<$ty>(
                concat!($name, "\0").as_ptr().cast::<crate::ffi::c_char>(),
            )
        })
    }};
}

unsafe fn resolve_driver_from_ptr<T: Copy>(symbol: *const c_char) -> Option<T> {
    let handle = CUDA_DRIVER_HANDLE.load(Ordering::Relaxed);
    if !handle.is_null() {
        if let Some(real) = real_dlsym() {
            // SAFETY: forwarding handle/symbol to libc dlsym.
            let ptr = unsafe { real(handle, symbol) };
            if !ptr.is_null() {
                // SAFETY: caller guarantees function signature compatibility.
                return Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&ptr) });
            }
        }
    }

    // SAFETY: fallback path identical to existing RTLD_NEXT resolution.
    unsafe { resolve_next_from_ptr::<T>(symbol) }
}

fn intern_driver_func_name(symbol: &CStr) -> &'static str {
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
pub unsafe extern "C" fn cudatrace_driver_enter(func: *const c_char) -> *mut c_void {
    if func.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: generated wrappers pass a valid NUL-terminated static symbol name.
    let func = unsafe { CStr::from_ptr(func) };
    let func = intern_driver_func_name(func);
    let scope = Box::new(TraceScope::enter(TraceDomain::Driver, func, String::new()));
    Box::into_raw(scope).cast::<c_void>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudatrace_driver_exit(scope: *mut c_void, ret: CUresult) {
    if scope.is_null() {
        return;
    }

    // SAFETY: pointer originates from cudatrace_driver_enter and is consumed once on exit.
    let scope: Box<TraceScope> = unsafe { Box::from_raw(scope.cast::<TraceScope>()) };
    scope.finish(ret.to_string());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cudatrace_resolve_driver_symbol(symbol: *const c_char) -> *mut c_void {
    if symbol.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: caller passes a symbol name pointer suitable for dlsym-style lookup.
    unsafe { resolve_driver_from_ptr::<*mut c_void>(symbol) }.unwrap_or(std::ptr::null_mut())
}

#[cfg(has_generated_driver_wrappers)]
fn lookup_generated_driver_wrapper(symbol: *const c_char) -> *mut c_void {
    // SAFETY: symbol pointer is valid for this lookup call.
    unsafe { cudatrace_lookup_generated_driver_wrapper(symbol) }
}

#[cfg(not(has_generated_driver_wrappers))]
fn lookup_generated_driver_wrapper(_symbol: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

type CuInitFn = unsafe extern "C" fn(crate::ffi::c_uint) -> CUresult;
type CuDeviceGetFn = unsafe extern "C" fn(*mut CUdevice, crate::ffi::c_int) -> CUresult;
type CuDeviceGetNameFn = unsafe extern "C" fn(*mut c_char, crate::ffi::c_int, CUdevice) -> CUresult;
type CuCtxCreateV2Fn =
    unsafe extern "C" fn(*mut CUcontext, crate::ffi::c_uint, CUdevice) -> CUresult;
type CuCtxDestroyV2Fn = unsafe extern "C" fn(CUcontext) -> CUresult;
type CuCtxSynchronizeFn = unsafe extern "C" fn() -> CUresult;
type CuCtxGetCurrentFn = unsafe extern "C" fn(*mut CUcontext) -> CUresult;
type CuCtxSetCurrentFn = unsafe extern "C" fn(CUcontext) -> CUresult;
type CuCtxPushCurrentV2Fn = unsafe extern "C" fn(CUcontext) -> CUresult;
type CuCtxPopCurrentV2Fn = unsafe extern "C" fn(*mut CUcontext) -> CUresult;
type CuDevicePrimaryCtxRetainFn = unsafe extern "C" fn(*mut CUcontext, CUdevice) -> CUresult;
type CuDevicePrimaryCtxReleaseFn = unsafe extern "C" fn(CUdevice) -> CUresult;
type CuDevicePrimaryCtxResetFn = unsafe extern "C" fn(CUdevice) -> CUresult;
type CuDevicePrimaryCtxSetFlagsFn = unsafe extern "C" fn(CUdevice, crate::ffi::c_uint) -> CUresult;
type CuDevicePrimaryCtxGetStateFn =
    unsafe extern "C" fn(CUdevice, *mut crate::ffi::c_uint, *mut crate::ffi::c_int) -> CUresult;
type CuStreamCreateFn = unsafe extern "C" fn(*mut CUstream, crate::ffi::c_uint) -> CUresult;
type CuStreamCreateWithPriorityFn =
    unsafe extern "C" fn(*mut CUstream, crate::ffi::c_uint, crate::ffi::c_int) -> CUresult;
type CuStreamDestroyFn = unsafe extern "C" fn(CUstream) -> CUresult;
type CuEventCreateFn = unsafe extern "C" fn(*mut CUevent, crate::ffi::c_uint) -> CUresult;
type CuEventDestroyFn = unsafe extern "C" fn(CUevent) -> CUresult;
type CuEventRecordFn = unsafe extern "C" fn(CUevent, CUstream) -> CUresult;
type CuEventRecordWithFlagsFn =
    unsafe extern "C" fn(CUevent, CUstream, crate::ffi::c_uint) -> CUresult;
type CuEventSynchronizeFn = unsafe extern "C" fn(CUevent) -> CUresult;
type CuEventElapsedTimeFn = unsafe extern "C" fn(*mut f32, CUevent, CUevent) -> CUresult;
type CuModuleLoadFn = unsafe extern "C" fn(*mut CUmodule, *const c_char) -> CUresult;
type CuModuleGetFunctionFn =
    unsafe extern "C" fn(*mut CUfunction, CUmodule, *const c_char) -> CUresult;
type CuModuleUnloadFn = unsafe extern "C" fn(CUmodule) -> CUresult;
type CuLaunchKernelFn = unsafe extern "C" fn(
    CUfunction,
    crate::ffi::c_uint,
    crate::ffi::c_uint,
    crate::ffi::c_uint,
    crate::ffi::c_uint,
    crate::ffi::c_uint,
    crate::ffi::c_uint,
    crate::ffi::c_uint,
    CUstream,
    *mut *mut c_void,
    *mut *mut c_void,
) -> CUresult;
type CuMemcpyAsyncFn = unsafe extern "C" fn(CUdeviceptr, CUdeviceptr, usize, CUstream) -> CUresult;
type CuMemcpyHtoDAsyncV2Fn =
    unsafe extern "C" fn(CUdeviceptr, *const c_void, usize, CUstream) -> CUresult;
type CuMemcpyDtoHAsyncV2Fn =
    unsafe extern "C" fn(*mut c_void, CUdeviceptr, usize, CUstream) -> CUresult;
type CuMemcpyHtoDV2Fn = unsafe extern "C" fn(CUdeviceptr, *const c_void, usize) -> CUresult;
type CuMemcpyDtoHV2Fn = unsafe extern "C" fn(*mut c_void, CUdeviceptr, usize) -> CUresult;
type CuMemAllocV2Fn = unsafe extern "C" fn(*mut CUdeviceptr, usize) -> CUresult;
type CuMemFreeV2Fn = unsafe extern "C" fn(CUdeviceptr) -> CUresult;
type CuMemAllocHostV2Fn = unsafe extern "C" fn(*mut *mut c_void, usize) -> CUresult;
type CuMemFreeHostFn = unsafe extern "C" fn(*mut c_void) -> CUresult;
type CuGetProcAddressFn = unsafe extern "C" fn(
    *const c_char,
    *mut *mut c_void,
    crate::ffi::c_int,
    cuuint64_t,
) -> CUresult;
type CuGetProcAddressV2Fn = unsafe extern "C" fn(
    *const c_char,
    *mut *mut c_void,
    crate::ffi::c_int,
    cuuint64_t,
    *mut CUdriverProcAddressQueryResult,
) -> CUresult;
type DlSymFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;

fn real_cu_init() -> Option<CuInitFn> {
    resolve_next!(REAL_CU_INIT, "cuInit", CuInitFn)
}

fn real_cu_device_get() -> Option<CuDeviceGetFn> {
    resolve_next!(REAL_CU_DEVICE_GET, "cuDeviceGet", CuDeviceGetFn)
}

fn real_cu_device_get_name() -> Option<CuDeviceGetNameFn> {
    resolve_next!(
        REAL_CU_DEVICE_GET_NAME,
        "cuDeviceGetName",
        CuDeviceGetNameFn
    )
}

fn real_cu_ctx_create_v2() -> Option<CuCtxCreateV2Fn> {
    resolve_next!(REAL_CU_CTX_CREATE_V2, "cuCtxCreate_v2", CuCtxCreateV2Fn)
}

fn real_cu_ctx_destroy_v2() -> Option<CuCtxDestroyV2Fn> {
    resolve_next!(REAL_CU_CTX_DESTROY_V2, "cuCtxDestroy_v2", CuCtxDestroyV2Fn)
}

fn real_cu_ctx_synchronize() -> Option<CuCtxSynchronizeFn> {
    resolve_next!(
        REAL_CU_CTX_SYNCHRONIZE,
        "cuCtxSynchronize",
        CuCtxSynchronizeFn
    )
}

fn real_cu_ctx_get_current() -> Option<CuCtxGetCurrentFn> {
    resolve_next!(
        REAL_CU_CTX_GET_CURRENT,
        "cuCtxGetCurrent",
        CuCtxGetCurrentFn
    )
}

fn real_cu_ctx_set_current() -> Option<CuCtxSetCurrentFn> {
    resolve_next!(
        REAL_CU_CTX_SET_CURRENT,
        "cuCtxSetCurrent",
        CuCtxSetCurrentFn
    )
}

fn real_cu_ctx_push_current_v2() -> Option<CuCtxPushCurrentV2Fn> {
    resolve_next!(
        REAL_CU_CTX_PUSH_CURRENT_V2,
        "cuCtxPushCurrent_v2",
        CuCtxPushCurrentV2Fn
    )
}

fn real_cu_ctx_push_current() -> Option<CuCtxPushCurrentV2Fn> {
    resolve_next!(
        REAL_CU_CTX_PUSH_CURRENT,
        "cuCtxPushCurrent",
        CuCtxPushCurrentV2Fn
    )
}

fn real_cu_ctx_pop_current_v2() -> Option<CuCtxPopCurrentV2Fn> {
    resolve_next!(
        REAL_CU_CTX_POP_CURRENT_V2,
        "cuCtxPopCurrent_v2",
        CuCtxPopCurrentV2Fn
    )
}

fn real_cu_ctx_pop_current() -> Option<CuCtxPopCurrentV2Fn> {
    resolve_next!(
        REAL_CU_CTX_POP_CURRENT,
        "cuCtxPopCurrent",
        CuCtxPopCurrentV2Fn
    )
}

fn real_cu_device_primary_ctx_retain() -> Option<CuDevicePrimaryCtxRetainFn> {
    resolve_next!(
        REAL_CU_DEVICE_PRIMARY_CTX_RETAIN,
        "cuDevicePrimaryCtxRetain",
        CuDevicePrimaryCtxRetainFn
    )
}

fn real_cu_device_primary_ctx_release() -> Option<CuDevicePrimaryCtxReleaseFn> {
    resolve_next!(
        REAL_CU_DEVICE_PRIMARY_CTX_RELEASE,
        "cuDevicePrimaryCtxRelease",
        CuDevicePrimaryCtxReleaseFn
    )
}

fn real_cu_device_primary_ctx_reset() -> Option<CuDevicePrimaryCtxResetFn> {
    resolve_next!(
        REAL_CU_DEVICE_PRIMARY_CTX_RESET,
        "cuDevicePrimaryCtxReset",
        CuDevicePrimaryCtxResetFn
    )
}

fn real_cu_device_primary_ctx_set_flags() -> Option<CuDevicePrimaryCtxSetFlagsFn> {
    resolve_next!(
        REAL_CU_DEVICE_PRIMARY_CTX_SET_FLAGS,
        "cuDevicePrimaryCtxSetFlags",
        CuDevicePrimaryCtxSetFlagsFn
    )
}

fn real_cu_device_primary_ctx_set_flags_v2() -> Option<CuDevicePrimaryCtxSetFlagsFn> {
    resolve_next!(
        REAL_CU_DEVICE_PRIMARY_CTX_SET_FLAGS_V2,
        "cuDevicePrimaryCtxSetFlags_v2",
        CuDevicePrimaryCtxSetFlagsFn
    )
}

fn real_cu_device_primary_ctx_get_state() -> Option<CuDevicePrimaryCtxGetStateFn> {
    resolve_next!(
        REAL_CU_DEVICE_PRIMARY_CTX_GET_STATE,
        "cuDevicePrimaryCtxGetState",
        CuDevicePrimaryCtxGetStateFn
    )
}

fn real_cu_stream_create() -> Option<CuStreamCreateFn> {
    resolve_next!(REAL_CU_STREAM_CREATE, "cuStreamCreate", CuStreamCreateFn)
}

fn real_cu_stream_create_with_priority() -> Option<CuStreamCreateWithPriorityFn> {
    resolve_next!(
        REAL_CU_STREAM_CREATE_WITH_PRIORITY,
        "cuStreamCreateWithPriority",
        CuStreamCreateWithPriorityFn
    )
}

fn real_cu_stream_destroy() -> Option<CuStreamDestroyFn> {
    resolve_next!(REAL_CU_STREAM_DESTROY, "cuStreamDestroy", CuStreamDestroyFn)
}

fn real_cu_stream_destroy_v2() -> Option<CuStreamDestroyFn> {
    resolve_next!(
        REAL_CU_STREAM_DESTROY_V2,
        "cuStreamDestroy_v2",
        CuStreamDestroyFn
    )
}

fn real_cu_event_create() -> Option<CuEventCreateFn> {
    resolve_next!(REAL_CU_EVENT_CREATE, "cuEventCreate", CuEventCreateFn)
}

fn real_cu_event_destroy() -> Option<CuEventDestroyFn> {
    resolve_next!(REAL_CU_EVENT_DESTROY, "cuEventDestroy", CuEventDestroyFn)
}

fn real_cu_event_destroy_v2() -> Option<CuEventDestroyFn> {
    resolve_next!(
        REAL_CU_EVENT_DESTROY_V2,
        "cuEventDestroy_v2",
        CuEventDestroyFn
    )
}

fn real_cu_event_record() -> Option<CuEventRecordFn> {
    resolve_next!(REAL_CU_EVENT_RECORD, "cuEventRecord", CuEventRecordFn)
}

fn real_cu_event_record_with_flags() -> Option<CuEventRecordWithFlagsFn> {
    resolve_next!(
        REAL_CU_EVENT_RECORD_WITH_FLAGS,
        "cuEventRecordWithFlags",
        CuEventRecordWithFlagsFn
    )
}

fn real_cu_event_synchronize() -> Option<CuEventSynchronizeFn> {
    resolve_next!(
        REAL_CU_EVENT_SYNCHRONIZE,
        "cuEventSynchronize",
        CuEventSynchronizeFn
    )
}

fn real_cu_event_elapsed_time() -> Option<CuEventElapsedTimeFn> {
    resolve_next!(
        REAL_CU_EVENT_ELAPSED_TIME,
        "cuEventElapsedTime",
        CuEventElapsedTimeFn
    )
}

fn real_cu_event_elapsed_time_v2() -> Option<CuEventElapsedTimeFn> {
    resolve_next!(
        REAL_CU_EVENT_ELAPSED_TIME_V2,
        "cuEventElapsedTime_v2",
        CuEventElapsedTimeFn
    )
}

fn real_cu_memcpy_async() -> Option<CuMemcpyAsyncFn> {
    resolve_next!(REAL_CU_MEMCPY_ASYNC, "cuMemcpyAsync", CuMemcpyAsyncFn)
}

fn real_cu_memcpy_htod_async_v2() -> Option<CuMemcpyHtoDAsyncV2Fn> {
    resolve_next!(
        REAL_CU_MEMCPY_HTOD_ASYNC_V2,
        "cuMemcpyHtoDAsync_v2",
        CuMemcpyHtoDAsyncV2Fn
    )
}

fn real_cu_memcpy_htod_async() -> Option<CuMemcpyHtoDAsyncV2Fn> {
    resolve_next!(
        REAL_CU_MEMCPY_HTOD_ASYNC,
        "cuMemcpyHtoDAsync",
        CuMemcpyHtoDAsyncV2Fn
    )
}

fn real_cu_memcpy_dtoh_async_v2() -> Option<CuMemcpyDtoHAsyncV2Fn> {
    resolve_next!(
        REAL_CU_MEMCPY_DTOH_ASYNC_V2,
        "cuMemcpyDtoHAsync_v2",
        CuMemcpyDtoHAsyncV2Fn
    )
}

fn real_cu_memcpy_dtoh_async() -> Option<CuMemcpyDtoHAsyncV2Fn> {
    resolve_next!(
        REAL_CU_MEMCPY_DTOH_ASYNC,
        "cuMemcpyDtoHAsync",
        CuMemcpyDtoHAsyncV2Fn
    )
}

fn real_cu_module_load() -> Option<CuModuleLoadFn> {
    resolve_next!(REAL_CU_MODULE_LOAD, "cuModuleLoad", CuModuleLoadFn)
}

fn real_cu_module_get_function() -> Option<CuModuleGetFunctionFn> {
    resolve_next!(
        REAL_CU_MODULE_GET_FUNCTION,
        "cuModuleGetFunction",
        CuModuleGetFunctionFn
    )
}

fn real_cu_module_unload() -> Option<CuModuleUnloadFn> {
    resolve_next!(REAL_CU_MODULE_UNLOAD, "cuModuleUnload", CuModuleUnloadFn)
}

fn real_cu_launch_kernel() -> Option<CuLaunchKernelFn> {
    resolve_next!(REAL_CU_LAUNCH_KERNEL, "cuLaunchKernel", CuLaunchKernelFn)
}

fn real_cu_memcpy_htod_v2() -> Option<CuMemcpyHtoDV2Fn> {
    resolve_next!(REAL_CU_MEMCPY_HTOD_V2, "cuMemcpyHtoD_v2", CuMemcpyHtoDV2Fn)
}

fn real_cu_memcpy_dtoh_v2() -> Option<CuMemcpyDtoHV2Fn> {
    resolve_next!(REAL_CU_MEMCPY_DTOH_V2, "cuMemcpyDtoH_v2", CuMemcpyDtoHV2Fn)
}

fn real_cu_mem_alloc_v2() -> Option<CuMemAllocV2Fn> {
    resolve_next!(REAL_CU_MEM_ALLOC_V2, "cuMemAlloc_v2", CuMemAllocV2Fn)
}

fn real_cu_mem_free_v2() -> Option<CuMemFreeV2Fn> {
    resolve_next!(REAL_CU_MEM_FREE_V2, "cuMemFree_v2", CuMemFreeV2Fn)
}

fn real_cu_mem_alloc_host_v2() -> Option<CuMemAllocHostV2Fn> {
    resolve_next!(
        REAL_CU_MEM_ALLOC_HOST_V2,
        "cuMemAllocHost_v2",
        CuMemAllocHostV2Fn
    )
}

fn real_cu_mem_free_host() -> Option<CuMemFreeHostFn> {
    resolve_next!(REAL_CU_MEM_FREE_HOST, "cuMemFreeHost", CuMemFreeHostFn)
}

fn real_cu_get_proc_address() -> Option<CuGetProcAddressFn> {
    resolve_next!(
        REAL_CU_GET_PROC_ADDRESS,
        "cuGetProcAddress",
        CuGetProcAddressFn
    )
}

fn real_cu_get_proc_address_v2() -> Option<CuGetProcAddressV2Fn> {
    resolve_next!(
        REAL_CU_GET_PROC_ADDRESS_V2,
        "cuGetProcAddress_v2",
        CuGetProcAddressV2Fn
    )
}

fn real_dlsym() -> Option<DlSymFn> {
    static REAL_DLSYM: OnceLock<Option<DlSymFn>> = OnceLock::new();
    *REAL_DLSYM.get_or_init(|| unsafe {
        let ptr = crate::ffi::dlvsym(
            crate::ffi::RTLD_NEXT,
            c"dlsym".as_ptr(),
            c"GLIBC_2.2.5".as_ptr(),
        );
        if ptr.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy::<*mut c_void, DlSymFn>(&ptr))
        }
    })
}

fn wrapper_for_driver_symbol(symbol: &CStr) -> Option<*mut c_void> {
    let ptr = match symbol.to_bytes() {
        b"cuInit" => cuInit as *const () as *mut c_void,
        b"cuDeviceGet" => cuDeviceGet as *const () as *mut c_void,
        b"cuDeviceGetName" => cuDeviceGetName as *const () as *mut c_void,
        b"cuCtxCreate" | b"cuCtxCreate_v2" => cuCtxCreate_v2 as *const () as *mut c_void,
        b"cuCtxDestroy" | b"cuCtxDestroy_v2" => cuCtxDestroy_v2 as *const () as *mut c_void,
        b"cuCtxSynchronize" => cuCtxSynchronize as *const () as *mut c_void,
        b"cuCtxGetCurrent" => cuCtxGetCurrent as *const () as *mut c_void,
        b"cuCtxSetCurrent" => cuCtxSetCurrent as *const () as *mut c_void,
        b"cuCtxPushCurrent" | b"cuCtxPushCurrent_v2" => {
            cuCtxPushCurrent_v2 as *const () as *mut c_void
        }
        b"cuCtxPopCurrent" | b"cuCtxPopCurrent_v2" => {
            cuCtxPopCurrent_v2 as *const () as *mut c_void
        }
        b"cuDevicePrimaryCtxRetain" => cuDevicePrimaryCtxRetain as *const () as *mut c_void,
        b"cuDevicePrimaryCtxRelease" => cuDevicePrimaryCtxRelease as *const () as *mut c_void,
        b"cuDevicePrimaryCtxReset" => cuDevicePrimaryCtxReset as *const () as *mut c_void,
        b"cuDevicePrimaryCtxSetFlags" | b"cuDevicePrimaryCtxSetFlags_v2" => {
            cuDevicePrimaryCtxSetFlags as *const () as *mut c_void
        }
        b"cuDevicePrimaryCtxGetState" => cuDevicePrimaryCtxGetState as *const () as *mut c_void,
        b"cuStreamCreate" => cuStreamCreate as *const () as *mut c_void,
        b"cuStreamCreateWithPriority" => cuStreamCreateWithPriority as *const () as *mut c_void,
        b"cuStreamDestroy" | b"cuStreamDestroy_v2" => cuStreamDestroy as *const () as *mut c_void,
        b"cuEventCreate" => cuEventCreate as *const () as *mut c_void,
        b"cuEventDestroy" | b"cuEventDestroy_v2" => cuEventDestroy as *const () as *mut c_void,
        b"cuEventRecord" | b"cuEventRecord_ptsz" => cuEventRecord as *const () as *mut c_void,
        b"cuEventRecordWithFlags" | b"cuEventRecordWithFlags_ptsz" => {
            cuEventRecordWithFlags as *const () as *mut c_void
        }
        b"cuEventSynchronize" => cuEventSynchronize as *const () as *mut c_void,
        b"cuEventElapsedTime" | b"cuEventElapsedTime_v2" => {
            cuEventElapsedTime as *const () as *mut c_void
        }
        b"cuMemcpyAsync" | b"cuMemcpyAsync_ptsz" => cuMemcpyAsync as *const () as *mut c_void,
        b"cuMemcpyHtoDAsync" | b"cuMemcpyHtoDAsync_v2" => {
            cuMemcpyHtoDAsync_v2 as *const () as *mut c_void
        }
        b"cuMemcpyHtoDAsync_v2_ptsz" => cuMemcpyHtoDAsync_v2 as *const () as *mut c_void,
        b"cuMemcpyDtoHAsync" | b"cuMemcpyDtoHAsync_v2" => {
            cuMemcpyDtoHAsync_v2 as *const () as *mut c_void
        }
        b"cuMemcpyDtoHAsync_v2_ptsz" => cuMemcpyDtoHAsync_v2 as *const () as *mut c_void,
        b"cuModuleLoad" => cuModuleLoad as *const () as *mut c_void,
        b"cuModuleGetFunction" => cuModuleGetFunction as *const () as *mut c_void,
        b"cuModuleUnload" => cuModuleUnload as *const () as *mut c_void,
        b"cuLaunchKernel" => cuLaunchKernel as *const () as *mut c_void,
        b"cuMemcpyHtoD" | b"cuMemcpyHtoD_v2" => cuMemcpyHtoD_v2 as *const () as *mut c_void,
        b"cuMemcpyDtoH" | b"cuMemcpyDtoH_v2" => cuMemcpyDtoH_v2 as *const () as *mut c_void,
        b"cuMemAlloc" | b"cuMemAlloc_v2" => cuMemAlloc_v2 as *const () as *mut c_void,
        b"cuMemFree" | b"cuMemFree_v2" => cuMemFree_v2 as *const () as *mut c_void,
        b"cuMemAllocHost" | b"cuMemAllocHost_v2" => cuMemAllocHost_v2 as *const () as *mut c_void,
        b"cuMemFreeHost" => cuMemFreeHost as *const () as *mut c_void,
        b"cuGetProcAddress" => cuGetProcAddress as *const () as *mut c_void,
        b"cuGetProcAddress_v2" => cuGetProcAddress_v2 as *const () as *mut c_void,
        _ => std::ptr::null_mut(),
    };
    if !ptr.is_null() {
        return Some(ptr);
    }

    let generated = lookup_generated_driver_wrapper(symbol.as_ptr());
    if !generated.is_null() {
        return Some(generated);
    }

    None
}

unsafe fn patch_driver_proc(symbol: *const c_char, pfn: *mut *mut c_void) -> bool {
    if symbol.is_null() || pfn.is_null() {
        return false;
    }

    // SAFETY: cuGetProcAddress provides a valid symbol pointer.
    let symbol = unsafe { CStr::from_ptr(symbol) };
    if let Some(wrapper) = wrapper_for_driver_symbol(symbol) {
        // SAFETY: caller provides an output function-pointer storage.
        unsafe {
            *pfn = wrapper;
        }
        true
    } else {
        false
    }
}

fn dlsym_override(symbol: *const c_char) -> Option<*mut c_void> {
    if symbol.is_null() {
        return None;
    }

    // SAFETY: dlsym contract requires a valid symbol C string.
    let symbol = unsafe { CStr::from_ptr(symbol) };
    if let Some(wrapper) = wrapper_for_driver_symbol(symbol) {
        return Some(wrapper);
    }

    if let Some(wrapper) = crate::hook::cudart::wrapper_for_cudart_symbol(symbol) {
        return Some(wrapper);
    }

    None
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    let Some(real) = real_dlsym() else {
        return std::ptr::null_mut();
    };

    // SAFETY: forwarding to libc dlsym with original arguments.
    let real_ptr = unsafe { real(handle, symbol) };

    if handle != crate::ffi::RTLD_NEXT && !real_ptr.is_null() {
        if let Some(wrapper) = dlsym_override(symbol) {
            CUDA_DRIVER_HANDLE.store(handle, Ordering::Relaxed);
            return wrapper;
        }
    }

    real_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuGetProcAddress(
    symbol: *const c_char,
    pfn: *mut *mut c_void,
    cuda_version: crate::ffi::c_int,
    flags: cuuint64_t,
) -> CUresult {
    let ret = if let Some(real) = real_cu_get_proc_address() {
        // SAFETY: forwarding to real cuGetProcAddress ABI.
        unsafe { real(symbol, pfn, cuda_version, flags) }
    } else if let Some(real_v2) = real_cu_get_proc_address_v2() {
        // SAFETY: legacy signature is equivalent to v2 with null symbolStatus.
        unsafe { real_v2(symbol, pfn, cuda_version, flags, std::ptr::null_mut()) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    if ret == CUDA_SUCCESS {
        // SAFETY: output pointer comes from cuGetProcAddress ABI.
        let _ = unsafe { patch_driver_proc(symbol, pfn) };
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuGetProcAddress_v2(
    symbol: *const c_char,
    pfn: *mut *mut c_void,
    cuda_version: crate::ffi::c_int,
    flags: cuuint64_t,
    symbol_status: *mut CUdriverProcAddressQueryResult,
) -> CUresult {
    let ret = if let Some(real) = real_cu_get_proc_address_v2() {
        // SAFETY: forwarding to real cuGetProcAddress_v2 ABI.
        unsafe { real(symbol, pfn, cuda_version, flags, symbol_status) }
    } else if let Some(real_legacy) = real_cu_get_proc_address() {
        // SAFETY: legacy signature ignores symbolStatus output.
        unsafe { real_legacy(symbol, pfn, cuda_version, flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    if ret == CUDA_SUCCESS {
        // SAFETY: output pointer comes from cuGetProcAddress ABI.
        let _ = unsafe { patch_driver_proc(symbol, pfn) };
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuInit(flags: crate::ffi::c_uint) -> CUresult {
    let scope = TraceScope::enter(TraceDomain::Driver, "cuInit", format!("flags=0x{flags:x}"));

    let ret = if let Some(real) = real_cu_init() {
        unsafe { real(flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDeviceGet(
    device: *mut CUdevice,
    ordinal: crate::ffi::c_int,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDeviceGet",
        format!("device={device:p}, ordinal={ordinal}"),
    );

    let ret = if let Some(real) = real_cu_device_get() {
        unsafe { real(device, ordinal) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDeviceGetName(
    name: *mut c_char,
    len: crate::ffi::c_int,
    device: CUdevice,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDeviceGetName",
        format!("name={name:p}, len={len}, device={device}"),
    );

    let ret = if let Some(real) = real_cu_device_get_name() {
        unsafe { real(name, len, device) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxCreate_v2(
    pctx: *mut CUcontext,
    flags: crate::ffi::c_uint,
    dev: CUdevice,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuCtxCreate_v2",
        format!("pctx={pctx:p}, flags=0x{flags:x}, dev={dev}"),
    );

    let ret = if let Some(real) = real_cu_ctx_create_v2() {
        unsafe { real(pctx, flags, dev) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxDestroy_v2(ctx: CUcontext) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuCtxDestroy_v2",
        format!("ctx={ctx:p}"),
    );

    let ret = if let Some(real) = real_cu_ctx_destroy_v2() {
        unsafe { real(ctx) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxSynchronize() -> CUresult {
    let scope = TraceScope::enter(TraceDomain::Driver, "cuCtxSynchronize", String::new());

    let ret = if let Some(real) = real_cu_ctx_synchronize() {
        unsafe { real() }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxGetCurrent(pctx: *mut CUcontext) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuCtxGetCurrent",
        format!("pctx={pctx:p}"),
    );

    let ret = if let Some(real) = real_cu_ctx_get_current() {
        unsafe { real(pctx) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxSetCurrent(ctx: CUcontext) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuCtxSetCurrent",
        format!("ctx={ctx:p}"),
    );

    let ret = if let Some(real) = real_cu_ctx_set_current() {
        unsafe { real(ctx) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxPushCurrent_v2(ctx: CUcontext) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuCtxPushCurrent_v2",
        format!("ctx={ctx:p}"),
    );

    let ret = if let Some(real) = real_cu_ctx_push_current_v2().or_else(real_cu_ctx_push_current) {
        unsafe { real(ctx) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxPushCurrent(ctx: CUcontext) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuCtxPushCurrent_v2.
    unsafe { cuCtxPushCurrent_v2(ctx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxPopCurrent_v2(pctx: *mut CUcontext) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuCtxPopCurrent_v2",
        format!("pctx={pctx:p}"),
    );

    let ret = if let Some(real) = real_cu_ctx_pop_current_v2().or_else(real_cu_ctx_pop_current) {
        unsafe { real(pctx) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuCtxPopCurrent(pctx: *mut CUcontext) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuCtxPopCurrent_v2.
    unsafe { cuCtxPopCurrent_v2(pctx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDevicePrimaryCtxRetain(pctx: *mut CUcontext, dev: CUdevice) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDevicePrimaryCtxRetain",
        format!("pctx={pctx:p}, dev={dev}"),
    );

    let ret = if let Some(real) = real_cu_device_primary_ctx_retain() {
        unsafe { real(pctx, dev) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDevicePrimaryCtxRelease(dev: CUdevice) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDevicePrimaryCtxRelease",
        format!("dev={dev}"),
    );

    let ret = if let Some(real) = real_cu_device_primary_ctx_release() {
        unsafe { real(dev) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDevicePrimaryCtxReset(dev: CUdevice) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDevicePrimaryCtxReset",
        format!("dev={dev}"),
    );

    let ret = if let Some(real) = real_cu_device_primary_ctx_reset() {
        unsafe { real(dev) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDevicePrimaryCtxSetFlags(
    dev: CUdevice,
    flags: crate::ffi::c_uint,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDevicePrimaryCtxSetFlags",
        format!("dev={dev}, flags=0x{flags:x}"),
    );

    let ret = if let Some(real) =
        real_cu_device_primary_ctx_set_flags().or_else(real_cu_device_primary_ctx_set_flags_v2)
    {
        unsafe { real(dev, flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDevicePrimaryCtxSetFlags_v2(
    dev: CUdevice,
    flags: crate::ffi::c_uint,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuDevicePrimaryCtxSetFlags.
    unsafe { cuDevicePrimaryCtxSetFlags(dev, flags) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuDevicePrimaryCtxGetState(
    dev: CUdevice,
    flags: *mut crate::ffi::c_uint,
    active: *mut crate::ffi::c_int,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuDevicePrimaryCtxGetState",
        format!("dev={dev}, flags={flags:p}, active={active:p}"),
    );

    let ret = if let Some(real) = real_cu_device_primary_ctx_get_state() {
        unsafe { real(dev, flags, active) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuStreamCreate(
    stream: *mut CUstream,
    flags: crate::ffi::c_uint,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuStreamCreate",
        format!("stream={stream:p}, flags=0x{flags:x}"),
    );

    let ret = if let Some(real) = real_cu_stream_create() {
        unsafe { real(stream, flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuStreamCreateWithPriority(
    stream: *mut CUstream,
    flags: crate::ffi::c_uint,
    priority: crate::ffi::c_int,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuStreamCreateWithPriority",
        format!("stream={stream:p}, flags=0x{flags:x}, priority={priority}"),
    );

    let ret = if let Some(real) = real_cu_stream_create_with_priority() {
        unsafe { real(stream, flags, priority) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuStreamDestroy(stream: CUstream) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuStreamDestroy",
        format!("stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cu_stream_destroy().or_else(real_cu_stream_destroy_v2) {
        unsafe { real(stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuStreamDestroy_v2(stream: CUstream) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuStreamDestroy.
    unsafe { cuStreamDestroy(stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventCreate(event: *mut CUevent, flags: crate::ffi::c_uint) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuEventCreate",
        format!("event={event:p}, flags=0x{flags:x}"),
    );

    let ret = if let Some(real) = real_cu_event_create() {
        unsafe { real(event, flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventDestroy(event: CUevent) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuEventDestroy",
        format!("event={event:p}"),
    );

    let ret = if let Some(real) = real_cu_event_destroy().or_else(real_cu_event_destroy_v2) {
        unsafe { real(event) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventDestroy_v2(event: CUevent) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuEventDestroy.
    unsafe { cuEventDestroy(event) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventRecord(event: CUevent, stream: CUstream) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuEventRecord",
        format!("event={event:p}, stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cu_event_record() {
        unsafe { real(event, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventRecord_ptsz(event: CUevent, stream: CUstream) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuEventRecord.
    unsafe { cuEventRecord(event, stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventRecordWithFlags(
    event: CUevent,
    stream: CUstream,
    flags: crate::ffi::c_uint,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuEventRecordWithFlags",
        format!("event={event:p}, stream={stream:p}, flags=0x{flags:x}"),
    );

    let ret = if let Some(real) = real_cu_event_record_with_flags() {
        unsafe { real(event, stream, flags) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventRecordWithFlags_ptsz(
    event: CUevent,
    stream: CUstream,
    flags: crate::ffi::c_uint,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuEventRecordWithFlags.
    unsafe { cuEventRecordWithFlags(event, stream, flags) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventSynchronize(event: CUevent) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuEventSynchronize",
        format!("event={event:p}"),
    );

    let ret = if let Some(real) = real_cu_event_synchronize() {
        unsafe { real(event) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventElapsedTime(
    milliseconds: *mut f32,
    start: CUevent,
    end: CUevent,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuEventElapsedTime",
        format!("ms={milliseconds:p}, start={start:p}, end={end:p}"),
    );

    let ret =
        if let Some(real) = real_cu_event_elapsed_time().or_else(real_cu_event_elapsed_time_v2) {
            unsafe { real(milliseconds, start, end) }
        } else {
            CUDA_ERROR_UNKNOWN
        };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuEventElapsedTime_v2(
    milliseconds: *mut f32,
    start: CUevent,
    end: CUevent,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuEventElapsedTime.
    unsafe { cuEventElapsedTime(milliseconds, start, end) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuModuleLoad(module: *mut CUmodule, fname: *const c_char) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuModuleLoad",
        format!("module={module:p}, fname={fname:p}"),
    );

    let ret = if let Some(real) = real_cu_module_load() {
        unsafe { real(module, fname) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuModuleGetFunction(
    hfunc: *mut CUfunction,
    module: CUmodule,
    name: *const c_char,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuModuleGetFunction",
        format!("hfunc={hfunc:p}, module={module:p}, name={name:p}"),
    );

    let ret = if let Some(real) = real_cu_module_get_function() {
        unsafe { real(hfunc, module, name) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuModuleUnload(module: CUmodule) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuModuleUnload",
        format!("module={module:p}"),
    );

    let ret = if let Some(real) = real_cu_module_unload() {
        unsafe { real(module) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuLaunchKernel(
    f: CUfunction,
    grid_x: crate::ffi::c_uint,
    grid_y: crate::ffi::c_uint,
    grid_z: crate::ffi::c_uint,
    block_x: crate::ffi::c_uint,
    block_y: crate::ffi::c_uint,
    block_z: crate::ffi::c_uint,
    shared_mem_bytes: crate::ffi::c_uint,
    hstream: CUstream,
    kernel_params: *mut *mut c_void,
    extra: *mut *mut c_void,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuLaunchKernel",
        format!(
            "f={f:p}, grid=({grid_x},{grid_y},{grid_z}), block=({block_x},{block_y},{block_z}), sharedMem={shared_mem_bytes}, stream={hstream:p}, params={kernel_params:p}, extra={extra:p}"
        ),
    );

    let ret = if let Some(real) = real_cu_launch_kernel() {
        unsafe {
            real(
                f,
                grid_x,
                grid_y,
                grid_z,
                block_x,
                block_y,
                block_z,
                shared_mem_bytes,
                hstream,
                kernel_params,
                extra,
            )
        }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyAsync(
    dst: CUdeviceptr,
    src: CUdeviceptr,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemcpyAsync",
        format!("dst=0x{dst:x}, src=0x{src:x}, byte_count={byte_count}, stream={stream:p}"),
    );

    let ret = if let Some(real) = real_cu_memcpy_async() {
        unsafe { real(dst, src, byte_count, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyAsync_ptsz(
    dst: CUdeviceptr,
    src: CUdeviceptr,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuMemcpyAsync.
    unsafe { cuMemcpyAsync(dst, src, byte_count, stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyHtoDAsync_v2(
    dst_device: CUdeviceptr,
    src_host: *const c_void,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemcpyHtoDAsync_v2",
        format!(
            "dst_device=0x{dst_device:x}, src_host={src_host:p}, byte_count={byte_count}, stream={stream:p}"
        ),
    );

    let ret = if let Some(real) = real_cu_memcpy_htod_async_v2().or_else(real_cu_memcpy_htod_async)
    {
        unsafe { real(dst_device, src_host, byte_count, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyHtoDAsync(
    dst_device: CUdeviceptr,
    src_host: *const c_void,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuMemcpyHtoDAsync_v2 for current targets.
    unsafe { cuMemcpyHtoDAsync_v2(dst_device, src_host, byte_count, stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyHtoDAsync_v2_ptsz(
    dst_device: CUdeviceptr,
    src_host: *const c_void,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuMemcpyHtoDAsync_v2.
    unsafe { cuMemcpyHtoDAsync_v2(dst_device, src_host, byte_count, stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyDtoHAsync_v2(
    dst_host: *mut c_void,
    src_device: CUdeviceptr,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemcpyDtoHAsync_v2",
        format!(
            "dst_host={dst_host:p}, src_device=0x{src_device:x}, byte_count={byte_count}, stream={stream:p}"
        ),
    );

    let ret = if let Some(real) = real_cu_memcpy_dtoh_async_v2().or_else(real_cu_memcpy_dtoh_async)
    {
        unsafe { real(dst_host, src_device, byte_count, stream) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyDtoHAsync(
    dst_host: *mut c_void,
    src_device: CUdeviceptr,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuMemcpyDtoHAsync_v2 for current targets.
    unsafe { cuMemcpyDtoHAsync_v2(dst_host, src_device, byte_count, stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyDtoHAsync_v2_ptsz(
    dst_host: *mut c_void,
    src_device: CUdeviceptr,
    byte_count: usize,
    stream: CUstream,
) -> CUresult {
    // SAFETY: ABI and argument contract are identical to cuMemcpyDtoHAsync_v2.
    unsafe { cuMemcpyDtoHAsync_v2(dst_host, src_device, byte_count, stream) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyHtoD_v2(
    dst_device: CUdeviceptr,
    src_host: *const c_void,
    byte_count: usize,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemcpyHtoD_v2",
        format!("dst_device=0x{dst_device:x}, src_host={src_host:p}, byte_count={byte_count}"),
    );

    let ret = if let Some(real) = real_cu_memcpy_htod_v2() {
        unsafe { real(dst_device, src_host, byte_count) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemcpyDtoH_v2(
    dst_host: *mut c_void,
    src_device: CUdeviceptr,
    byte_count: usize,
) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemcpyDtoH_v2",
        format!("dst_host={dst_host:p}, src_device=0x{src_device:x}, byte_count={byte_count}"),
    );

    let ret = if let Some(real) = real_cu_memcpy_dtoh_v2() {
        unsafe { real(dst_host, src_device, byte_count) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemAlloc_v2(dptr: *mut CUdeviceptr, bytesize: usize) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemAlloc_v2",
        format!("dptr={dptr:p}, bytesize={bytesize}"),
    );

    let ret = if let Some(real) = real_cu_mem_alloc_v2() {
        unsafe { real(dptr, bytesize) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemFree_v2(dptr: CUdeviceptr) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemFree_v2",
        format!("dptr=0x{dptr:x}"),
    );

    let ret = if let Some(real) = real_cu_mem_free_v2() {
        unsafe { real(dptr) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemAllocHost_v2(pp: *mut *mut c_void, bytesize: usize) -> CUresult {
    let scope = TraceScope::enter(
        TraceDomain::Driver,
        "cuMemAllocHost_v2",
        format!("pp={pp:p}, bytesize={bytesize}"),
    );

    let ret = if let Some(real) = real_cu_mem_alloc_host_v2() {
        unsafe { real(pp, bytesize) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cuMemFreeHost(p: *mut c_void) -> CUresult {
    let scope = TraceScope::enter(TraceDomain::Driver, "cuMemFreeHost", format!("p={p:p}"));

    let ret = if let Some(real) = real_cu_mem_free_host() {
        unsafe { real(p) }
    } else {
        CUDA_ERROR_UNKNOWN
    };

    scope.finish(ret.to_string());
    ret
}

#[cfg(test)]
mod tests {
    use super::dlsym_override;
    use super::wrapper_for_driver_symbol;

    #[test]
    fn wrapper_table_covers_core_driver_symbols() {
        assert!(wrapper_for_driver_symbol(c"cuInit").is_some());
        assert!(wrapper_for_driver_symbol(c"cuMemAlloc_v2").is_some());
        assert!(wrapper_for_driver_symbol(c"cuGetProcAddress_v2").is_some());
        assert!(wrapper_for_driver_symbol(c"cuUnknownSymbol").is_none());
    }

    #[test]
    fn dlsym_override_recognizes_known_symbol() {
        assert!(dlsym_override(c"cuInit".as_ptr()).is_some());
        assert!(dlsym_override(c"cudaMalloc".as_ptr()).is_some());
        assert!(dlsym_override(c"not_exists".as_ptr()).is_none());
    }
}
