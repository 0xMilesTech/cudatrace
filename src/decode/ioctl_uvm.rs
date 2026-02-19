use crate::config::IoctlDecodeMode;
use crate::decode::ioctl::{DecodeSummary, IoctlMeta, hex_bytes, read_bytes, read_pod};
use crate::decode::ioctl_json::{JsonObject, json_quote};
use std::cmp::min;
use std::ffi::c_void;

const UVM_INITIALIZE: u64 = 0x30000001;
const UVM_DEINITIALIZE: u64 = 0x30000002;

const UVM_RESERVE_VA: u64 = 1;
const UVM_RELEASE_VA: u64 = 2;
const UVM_REGION_COMMIT: u64 = 3;
const UVM_REGION_DECOMMIT: u64 = 4;
const UVM_REGION_SET_STREAM: u64 = 5;
const UVM_SET_STREAM_RUNNING: u64 = 6;
const UVM_SET_STREAM_STOPPED: u64 = 7;
const UVM_ADD_SESSION: u64 = 10;
const UVM_ENABLE_COUNTERS: u64 = 12;
const UVM_MAP_COUNTER: u64 = 13;
const UVM_CREATE_EVENT_QUEUE: u64 = 14;
const UVM_REMOVE_EVENT_QUEUE: u64 = 15;
const UVM_MAP_EVENT_QUEUE: u64 = 16;
const UVM_EVENT_CTRL: u64 = 17;
const UVM_GET_GPU_UUID_TABLE: u64 = 20;
const UVM_REGION_SET_BACKING: u64 = 21;
const UVM_REGION_UNSET_BACKING: u64 = 22;
const UVM_CREATE_RANGE_GROUP: u64 = 23;
const UVM_DESTROY_RANGE_GROUP: u64 = 24;
const UVM_REGISTER_GPU_VASPACE: u64 = 25;
const UVM_UNREGISTER_GPU_VASPACE: u64 = 26;
const UVM_REGISTER_CHANNEL: u64 = 27;
const UVM_UNREGISTER_CHANNEL: u64 = 28;
const UVM_ENABLE_PEER_ACCESS: u64 = 29;
const UVM_DISABLE_PEER_ACCESS: u64 = 30;
const UVM_SET_RANGE_GROUP: u64 = 31;
const UVM_MAP_EXTERNAL_ALLOCATION: u64 = 33;
const UVM_FREE: u64 = 34;
const UVM_MEM_MAP: u64 = 35;
const UVM_REGISTER_GPU: u64 = 37;
const UVM_UNREGISTER_GPU: u64 = 38;
const UVM_PAGEABLE_MEM_ACCESS: u64 = 39;
const UVM_PREVENT_MIGRATION_RANGE_GROUPS: u64 = 40;
const UVM_ALLOW_MIGRATION_RANGE_GROUPS: u64 = 41;
const UVM_SET_PREFERRED_LOCATION: u64 = 42;
const UVM_UNSET_PREFERRED_LOCATION: u64 = 43;
const UVM_ENABLE_READ_DUPLICATION: u64 = 44;
const UVM_DISABLE_READ_DUPLICATION: u64 = 45;
const UVM_SET_ACCESSED_BY: u64 = 46;
const UVM_UNSET_ACCESSED_BY: u64 = 47;
const UVM_MIGRATE: u64 = 51;
const UVM_MIGRATE_RANGE_GROUP: u64 = 53;
const UVM_ENABLE_SYSTEM_WIDE_ATOMICS: u64 = 54;
const UVM_DISABLE_SYSTEM_WIDE_ATOMICS: u64 = 55;
const UVM_TOOLS_INIT_EVENT_TRACKER: u64 = 56;
const UVM_TOOLS_SET_NOTIFICATION_THRESHOLD: u64 = 57;
const UVM_TOOLS_EVENT_QUEUE_ENABLE_EVENTS: u64 = 58;
const UVM_TOOLS_EVENT_QUEUE_DISABLE_EVENTS: u64 = 59;
const UVM_TOOLS_ENABLE_COUNTERS: u64 = 60;
const UVM_TOOLS_DISABLE_COUNTERS: u64 = 61;
const UVM_TOOLS_READ_PROCESS_MEMORY: u64 = 62;
const UVM_TOOLS_WRITE_PROCESS_MEMORY: u64 = 63;
const UVM_TOOLS_GET_PROCESSOR_UUID_TABLE: u64 = 64;
const UVM_MAP_DYNAMIC_PARALLELISM_REGION: u64 = 65;
const UVM_UNMAP_EXTERNAL: u64 = 66;
const UVM_TOOLS_FLUSH_EVENTS: u64 = 67;
const UVM_ALLOC_SEMAPHORE_POOL: u64 = 68;
const UVM_CLEAN_UP_ZOMBIE_RESOURCES: u64 = 69;
const UVM_PAGEABLE_MEM_ACCESS_ON_GPU: u64 = 70;
const UVM_POPULATE_PAGEABLE: u64 = 71;
const UVM_VALIDATE_VA_RANGE: u64 = 72;
const UVM_CREATE_EXTERNAL_RANGE: u64 = 73;
const UVM_MAP_EXTERNAL_SPARSE: u64 = 74;
const UVM_MM_INITIALIZE: u64 = 75;
const UVM_TOOLS_INIT_EVENT_TRACKER_V2: u64 = 76;
const UVM_TOOLS_GET_PROCESSOR_UUID_TABLE_V2: u64 = 77;
const UVM_ALLOC_DEVICE_P2P: u64 = 78;
const UVM_CLEAR_ALL_ACCESS_COUNTERS: u64 = 79;
const UVM_DISCARD: u64 = 80;
const UVM_IS_8_SUPPORTED: u64 = 2047;

const UVM_MAX_STREAMS_PER_IOCTL_CALL: usize = 32;
const UVM_MAX_COUNTERS_PER_IOCTL_CALL: usize = 32;
const UVM_MAX_RANGE_GROUPS_PER_IOCTL_CALL: usize = 32;
const UVM_MAX_GPUS_V1: usize = 32;
const UVM_MAX_GPUS: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmInitializeParams {
    flags: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmReserveVaParams {
    requested_base: u64,
    length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegionCommitParams {
    requested_base: u64,
    length: u64,
    stream_id: u64,
    gpu_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegionSetStreamParams {
    requested_base: u64,
    length: u64,
    new_stream_id: u64,
    gpu_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmSetStreamRunningParams {
    stream_id: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmSetStreamStoppedParams {
    stream_id_array: [u64; UVM_MAX_STREAMS_PER_IOCTL_CALL],
    n_streams: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmAddSessionParams {
    pid_target: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmCounterConfig {
    scope: u32,
    name: u32,
    gpu_uuid: [u8; 16],
    state: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmEnableCountersParams {
    config: [UvmCounterConfig; UVM_MAX_COUNTERS_PER_IOCTL_CALL],
    count: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMapCounterParams {
    scope: u32,
    counter_name: u32,
    gpu_uuid: [u8; 16],
    addr: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmCreateEventQueueParams {
    event_queue_index: u32,
    queue_size: u64,
    notification_count: u64,
    notification_handle: u64,
    timestamp_type: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRemoveEventQueueParams {
    event_queue_index: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMapEventQueueParams {
    event_queue_index: u32,
    user_ro_data_addr: u64,
    user_rw_data_addr: u64,
    read_index_addr: u64,
    write_index_addr: u64,
    queue_buffer_addr: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmEventCtrlParams {
    event_queue_index: u32,
    event_type: i32,
    enable: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmGetGpuUuidTableParams {
    gpu_uuid_array: [[u8; 16]; UVM_MAX_GPUS_V1],
    valid_count: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegionSetBackingParams {
    gpu_uuid: [u8; 16],
    h_allocation: u32,
    va_addr: u64,
    region_length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegionUnsetBackingParams {
    va_addr: u64,
    region_length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmCreateRangeGroupParams {
    range_group_id: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmDestroyRangeGroupParams {
    range_group_id: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegisterGpuVaspaceParams {
    gpu_uuid: [u8; 16],
    rm_ctrl_fd: i32,
    h_client: u32,
    h_vaspace: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmUnregisterGpuVaspaceParams {
    gpu_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegisterChannelParams {
    gpu_uuid: [u8; 16],
    rm_ctrl_fd: i32,
    h_client: u32,
    h_channel: u32,
    base: u64,
    length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmUnregisterChannelParams {
    h_client: u32,
    h_channel: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmPeerAccessParams {
    gpu_uuid_a: [u8; 16],
    gpu_uuid_b: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmSetRangeGroupParams {
    range_group_id: u64,
    requested_base: u64,
    length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmGpuMappingAttributes {
    gpu_uuid: [u8; 16],
    gpu_mapping_type: u32,
    gpu_caching_type: u32,
    gpu_format_type: u32,
    gpu_element_bits: u32,
    gpu_compression_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMapExternalAllocationParams {
    base: u64,
    length: u64,
    offset: u64,
    per_gpu_attributes: [UvmGpuMappingAttributes; UVM_MAX_GPUS],
    gpu_attributes_count: u64,
    rm_ctrl_fd: i32,
    h_client: u32,
    h_memory: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmFreeParams {
    base: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMemMapParams {
    region_base: u64,
    region_length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRegisterGpuParams {
    gpu_uuid: [u8; 16],
    numa_enabled: u8,
    _pad0: [u8; 3],
    numa_node_id: i32,
    rm_ctrl_fd: i32,
    h_client: u32,
    h_smc_part_ref: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmUnregisterGpuParams {
    gpu_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmPageableMemAccessParams {
    pageable_mem_access: u8,
    _pad0: [u8; 3],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmRangeGroupsParams {
    range_group_ids: [u64; UVM_MAX_RANGE_GROUPS_PER_IOCTL_CALL],
    num_group_ids: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmSetPreferredLocationParams {
    requested_base: u64,
    length: u64,
    preferred_location: [u8; 16],
    preferred_cpu_numa_node: i32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmSetAccessedByParams {
    requested_base: u64,
    length: u64,
    accessed_by_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMigrateParams {
    base: u64,
    length: u64,
    destination_uuid: [u8; 16],
    flags: u32,
    semaphore_address: u64,
    semaphore_payload: u32,
    cpu_numa_node: i32,
    user_space_start: u64,
    user_space_length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMigrateRangeGroupParams {
    range_group_id: u64,
    destination_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmGpuUuidParams {
    gpu_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmToolsInitEventTrackerParams {
    queue_buffer: u64,
    queue_buffer_size: u64,
    control_buffer: u64,
    processor: [u8; 16],
    all_processors: u32,
    uvm_fd: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmToolsSetNotificationThresholdParams {
    notification_threshold: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmToolsFlagsParams {
    flags: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmToolsReadProcessMemoryParams {
    buffer: u64,
    size: u64,
    target_va: u64,
    bytes_read: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmToolsWriteProcessMemoryParams {
    buffer: u64,
    size: u64,
    target_va: u64,
    bytes_written: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmToolsGetProcessorUuidTableParams {
    table_ptr: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmBaseLengthGpuUuidParams {
    base: u64,
    length: u64,
    gpu_uuid: [u8; 16],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmAllocSemaphorePoolParams {
    base: u64,
    length: u64,
    per_gpu_attributes: [UvmGpuMappingAttributes; UVM_MAX_GPUS],
    gpu_attributes_count: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmStatusParams {
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmPageableMemAccessOnGpuParams {
    gpu_uuid: [u8; 16],
    pageable_mem_access: u8,
    _pad0: [u8; 3],
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmPopulatePageableParams {
    base: u64,
    length: u64,
    flags: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmValidateVaRangeParams {
    base: u64,
    length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmCreateExternalRangeParams {
    base: u64,
    length: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmMmInitializeParams {
    uvm_fd: i32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmAllocDeviceP2pParams {
    base: u64,
    length: u64,
    offset: u64,
    gpu_uuid: [u8; 16],
    rm_ctrl_fd: i32,
    h_client: u32,
    h_memory: u32,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmDiscardParams {
    base: u64,
    length: u64,
    flags: u64,
    rm_status: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UvmIs8SupportedParams {
    is8_supported: u32,
    rm_status: i32,
}

pub fn decode_uvm_ioctl(
    meta: &IoctlMeta,
    arg_ptr: *mut c_void,
    decode_mode: IoctlDecodeMode,
    max_blob: usize,
) -> Option<DecodeSummary> {
    let mut root = JsonObject::new();

    if decode_mode == IoctlDecodeMode::Header {
        root.field_str("status", "header");
        return Some(DecodeSummary {
            legacy: String::new(),
            json: root.finish(),
        });
    }

    let raw_cmd = normalize_uvm_cmd(meta.raw as u64);
    if raw_cmd == UVM_DEINITIALIZE {
        root.field_str("status", "ok");
        return Some(DecodeSummary {
            legacy: String::new(),
            json: root.finish(),
        });
    }

    if arg_ptr.is_null() {
        root.field_str("status", "arg_null");
        return Some(DecodeSummary {
            legacy: String::new(),
            json: root.finish(),
        });
    }

    match raw_cmd {
        UVM_INITIALIZE => decode_initialize(arg_ptr, &mut root),
        UVM_RESERVE_VA => decode_reserve_va(arg_ptr, &mut root),
        UVM_RELEASE_VA => decode_release_va(arg_ptr, &mut root),
        UVM_REGION_COMMIT => decode_region_commit(arg_ptr, &mut root),
        UVM_REGION_DECOMMIT => decode_region_decommit(arg_ptr, &mut root),
        UVM_REGION_SET_STREAM => decode_region_set_stream(arg_ptr, &mut root),
        UVM_SET_STREAM_RUNNING => decode_set_stream_running(arg_ptr, &mut root),
        UVM_SET_STREAM_STOPPED => decode_set_stream_stopped(arg_ptr, &mut root),
        UVM_ADD_SESSION => decode_add_session(arg_ptr, &mut root),
        UVM_ENABLE_COUNTERS => decode_enable_counters(arg_ptr, &mut root),
        UVM_MAP_COUNTER => decode_map_counter(arg_ptr, &mut root),
        UVM_CREATE_EVENT_QUEUE => decode_create_event_queue(arg_ptr, &mut root),
        UVM_REMOVE_EVENT_QUEUE => decode_remove_event_queue(arg_ptr, &mut root),
        UVM_MAP_EVENT_QUEUE => decode_map_event_queue(arg_ptr, &mut root),
        UVM_EVENT_CTRL => decode_event_ctrl(arg_ptr, &mut root),
        UVM_GET_GPU_UUID_TABLE => decode_get_gpu_uuid_table(arg_ptr, &mut root),
        UVM_REGION_SET_BACKING => decode_region_set_backing(arg_ptr, &mut root),
        UVM_REGION_UNSET_BACKING => decode_region_unset_backing(arg_ptr, &mut root),
        UVM_CREATE_RANGE_GROUP => decode_create_range_group(arg_ptr, &mut root),
        UVM_DESTROY_RANGE_GROUP => decode_destroy_range_group(arg_ptr, &mut root),
        UVM_REGISTER_GPU_VASPACE => decode_register_gpu_vaspace(arg_ptr, &mut root),
        UVM_UNREGISTER_GPU_VASPACE => decode_unregister_gpu_vaspace(arg_ptr, &mut root),
        UVM_REGISTER_CHANNEL => decode_register_channel(arg_ptr, &mut root),
        UVM_UNREGISTER_CHANNEL => decode_unregister_channel(arg_ptr, &mut root),
        UVM_ENABLE_PEER_ACCESS => decode_enable_peer_access(arg_ptr, &mut root),
        UVM_DISABLE_PEER_ACCESS => decode_disable_peer_access(arg_ptr, &mut root),
        UVM_SET_RANGE_GROUP => decode_set_range_group(arg_ptr, &mut root),
        UVM_MAP_EXTERNAL_ALLOCATION => decode_map_external_allocation(arg_ptr, &mut root),
        UVM_FREE => decode_free(arg_ptr, &mut root),
        UVM_MEM_MAP => decode_mem_map(arg_ptr, &mut root),
        UVM_REGISTER_GPU => decode_register_gpu(arg_ptr, &mut root),
        UVM_UNREGISTER_GPU => decode_unregister_gpu(arg_ptr, &mut root),
        UVM_PAGEABLE_MEM_ACCESS => decode_pageable_mem_access(arg_ptr, &mut root),
        UVM_PREVENT_MIGRATION_RANGE_GROUPS => {
            decode_prevent_migration_range_groups(arg_ptr, &mut root)
        }
        UVM_ALLOW_MIGRATION_RANGE_GROUPS => decode_allow_migration_range_groups(arg_ptr, &mut root),
        UVM_SET_PREFERRED_LOCATION => decode_set_preferred_location(arg_ptr, &mut root),
        UVM_UNSET_PREFERRED_LOCATION => decode_unset_preferred_location(arg_ptr, &mut root),
        UVM_ENABLE_READ_DUPLICATION => decode_enable_read_duplication(arg_ptr, &mut root),
        UVM_DISABLE_READ_DUPLICATION => decode_disable_read_duplication(arg_ptr, &mut root),
        UVM_SET_ACCESSED_BY => decode_set_accessed_by(arg_ptr, &mut root),
        UVM_UNSET_ACCESSED_BY => decode_unset_accessed_by(arg_ptr, &mut root),
        UVM_MIGRATE => decode_migrate(arg_ptr, &mut root),
        UVM_MIGRATE_RANGE_GROUP => decode_migrate_range_group(arg_ptr, &mut root),
        UVM_ENABLE_SYSTEM_WIDE_ATOMICS => decode_enable_system_wide_atomics(arg_ptr, &mut root),
        UVM_DISABLE_SYSTEM_WIDE_ATOMICS => decode_disable_system_wide_atomics(arg_ptr, &mut root),
        UVM_TOOLS_INIT_EVENT_TRACKER | UVM_TOOLS_INIT_EVENT_TRACKER_V2 => {
            decode_tools_init_event_tracker(arg_ptr, &mut root)
        }
        UVM_TOOLS_SET_NOTIFICATION_THRESHOLD => {
            decode_tools_set_notification_threshold(arg_ptr, &mut root)
        }
        UVM_TOOLS_EVENT_QUEUE_ENABLE_EVENTS => {
            decode_tools_event_queue_enable_events(arg_ptr, &mut root)
        }
        UVM_TOOLS_EVENT_QUEUE_DISABLE_EVENTS => {
            decode_tools_event_queue_disable_events(arg_ptr, &mut root)
        }
        UVM_TOOLS_ENABLE_COUNTERS => decode_tools_enable_counters(arg_ptr, &mut root),
        UVM_TOOLS_DISABLE_COUNTERS => decode_tools_disable_counters(arg_ptr, &mut root),
        UVM_TOOLS_READ_PROCESS_MEMORY => decode_tools_read_process_memory(arg_ptr, &mut root),
        UVM_TOOLS_WRITE_PROCESS_MEMORY => decode_tools_write_process_memory(arg_ptr, &mut root),
        UVM_TOOLS_GET_PROCESSOR_UUID_TABLE | UVM_TOOLS_GET_PROCESSOR_UUID_TABLE_V2 => {
            decode_tools_get_processor_uuid_table(arg_ptr, &mut root)
        }
        UVM_MAP_DYNAMIC_PARALLELISM_REGION => {
            decode_map_dynamic_parallelism_region(arg_ptr, &mut root)
        }
        UVM_UNMAP_EXTERNAL => decode_unmap_external(arg_ptr, &mut root),
        UVM_TOOLS_FLUSH_EVENTS => decode_tools_flush_events(arg_ptr, &mut root),
        UVM_ALLOC_SEMAPHORE_POOL => decode_alloc_semaphore_pool(arg_ptr, &mut root),
        UVM_CLEAN_UP_ZOMBIE_RESOURCES => decode_clean_up_zombie_resources(arg_ptr, &mut root),
        UVM_PAGEABLE_MEM_ACCESS_ON_GPU => decode_pageable_mem_access_on_gpu(arg_ptr, &mut root),
        UVM_POPULATE_PAGEABLE => decode_populate_pageable(arg_ptr, &mut root),
        UVM_VALIDATE_VA_RANGE => decode_validate_va_range(arg_ptr, &mut root),
        UVM_CREATE_EXTERNAL_RANGE => decode_create_external_range(arg_ptr, &mut root),
        UVM_MAP_EXTERNAL_SPARSE => decode_map_external_sparse(arg_ptr, &mut root),
        UVM_MM_INITIALIZE => decode_mm_initialize(arg_ptr, &mut root),
        UVM_ALLOC_DEVICE_P2P => decode_alloc_device_p2p(arg_ptr, &mut root),
        UVM_CLEAR_ALL_ACCESS_COUNTERS => decode_clear_all_access_counters(arg_ptr, &mut root),
        UVM_DISCARD => decode_discard(arg_ptr, &mut root),
        UVM_IS_8_SUPPORTED => decode_is8_supported(arg_ptr, &mut root),
        _ => decode_unknown(arg_ptr, max_blob, &mut root),
    }

    Some(DecodeSummary {
        legacy: String::new(),
        json: root.finish(),
    })
}

fn with_params<T: Copy, F>(arg_ptr: *mut c_void, root: &mut JsonObject, on_ok: F)
where
    F: FnOnce(&T, &mut JsonObject),
{
    if let Some(params) = unsafe { read_pod::<T>(arg_ptr as usize) } {
        on_ok(&params, root);
    } else {
        root.field_str("status", "arg_unreadable");
    }
}

fn decode_initialize(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmInitializeParams, _>(arg_ptr, root, |params, root| {
        root.field_str("flags", &format!("0x{:x}", params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_reserve_va(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmReserveVaParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_release_va(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmReserveVaParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_region_commit(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegionCommitParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_str("streamId", &hex_u64(params.stream_id));
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_region_decommit(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmReserveVaParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_region_set_stream(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegionSetStreamParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_str("newStreamId", &hex_u64(params.new_stream_id));
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_set_stream_running(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmSetStreamRunningParams, _>(arg_ptr, root, |params, root| {
        root.field_str("streamId", &hex_u64(params.stream_id));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_set_stream_stopped(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmSetStreamStoppedParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) = clamp_count(params.n_streams, UVM_MAX_STREAMS_PER_IOCTL_CALL);
        root.field_u64("nStreams", params.n_streams);
        root.field_raw(
            "streamIds",
            &json_u64_array(&params.stream_id_array[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_add_session(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmAddSessionParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("pidTarget", params.pid_target as u64);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_enable_counters(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmEnableCountersParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) = clamp_count(params.count as u64, UVM_MAX_COUNTERS_PER_IOCTL_CALL);
        root.field_u64("count", params.count as u64);
        root.field_raw(
            "config",
            &json_counter_config_array(&params.config[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_map_counter(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMapCounterParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("scope", params.scope as u64);
        root.field_u64("counterName", params.counter_name as u64);
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_str("addr", &hex_u64(params.addr));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_create_event_queue(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmCreateEventQueueParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("eventQueueIndex", params.event_queue_index as u64);
        root.field_u64("queueSize", params.queue_size);
        root.field_u64("notificationCount", params.notification_count);
        root.field_u64("notificationHandle", params.notification_handle);
        root.field_u64("timeStampType", params.timestamp_type as u64);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_remove_event_queue(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRemoveEventQueueParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("eventQueueIndex", params.event_queue_index as u64);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_map_event_queue(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMapEventQueueParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("eventQueueIndex", params.event_queue_index as u64);
        root.field_str("userRODataAddr", &hex_u64(params.user_ro_data_addr));
        root.field_str("userRWDataAddr", &hex_u64(params.user_rw_data_addr));
        root.field_str("readIndexAddr", &hex_u64(params.read_index_addr));
        root.field_str("writeIndexAddr", &hex_u64(params.write_index_addr));
        root.field_str("queueBufferAddr", &hex_u64(params.queue_buffer_addr));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_event_ctrl(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmEventCtrlParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("eventQueueIndex", params.event_queue_index as u64);
        root.field_i64("eventType", params.event_type as i64);
        root.field_u64("enable", params.enable as u64);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_get_gpu_uuid_table(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmGetGpuUuidTableParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) = clamp_count(params.valid_count as u64, UVM_MAX_GPUS_V1);
        root.field_u64("validCount", params.valid_count as u64);
        root.field_raw(
            "gpuUuids",
            &json_uuid_array(&params.gpu_uuid_array[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_region_set_backing(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegionSetBackingParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_str("hAllocation", &hex_u32(params.h_allocation));
        root.field_str("vaAddr", &hex_u64(params.va_addr));
        root.field_u64("regionLength", params.region_length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_region_unset_backing(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegionUnsetBackingParams, _>(arg_ptr, root, |params, root| {
        root.field_str("vaAddr", &hex_u64(params.va_addr));
        root.field_u64("regionLength", params.region_length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_create_range_group(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmCreateRangeGroupParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("rangeGroupId", params.range_group_id);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_destroy_range_group(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmDestroyRangeGroupParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("rangeGroupId", params.range_group_id);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_register_gpu_vaspace(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegisterGpuVaspaceParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rmCtrlFd", params.rm_ctrl_fd as i64);
        root.field_str("hClient", &hex_u32(params.h_client));
        root.field_str("hVaSpace", &hex_u32(params.h_vaspace));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unregister_gpu_vaspace(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmUnregisterGpuVaspaceParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_register_channel(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegisterChannelParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rmCtrlFd", params.rm_ctrl_fd as i64);
        root.field_str("hClient", &hex_u32(params.h_client));
        root.field_str("hChannel", &hex_u32(params.h_channel));
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unregister_channel(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmUnregisterChannelParams, _>(arg_ptr, root, |params, root| {
        root.field_str("hClient", &hex_u32(params.h_client));
        root.field_str("hChannel", &hex_u32(params.h_channel));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_enable_peer_access(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmPeerAccessParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpuUuidA", &format_uuid(&params.gpu_uuid_a));
        root.field_str("gpuUuidB", &format_uuid(&params.gpu_uuid_b));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_disable_peer_access(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmPeerAccessParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpuUuidA", &format_uuid(&params.gpu_uuid_a));
        root.field_str("gpuUuidB", &format_uuid(&params.gpu_uuid_b));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_set_range_group(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmSetRangeGroupParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("rangeGroupId", params.range_group_id);
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_map_external_allocation(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMapExternalAllocationParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) = clamp_count(params.gpu_attributes_count, UVM_MAX_GPUS);
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_u64("offset", params.offset);
        root.field_u64("gpuAttributesCount", params.gpu_attributes_count);
        root.field_raw(
            "perGpuAttributes",
            &json_gpu_mapping_attributes_array(&params.per_gpu_attributes[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rmCtrlFd", params.rm_ctrl_fd as i64);
        root.field_str("hClient", &hex_u32(params.h_client));
        root.field_str("hMemory", &hex_u32(params.h_memory));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_free(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmFreeParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_mem_map(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMemMapParams, _>(arg_ptr, root, |params, root| {
        root.field_str("regionBase", &hex_u64(params.region_base));
        root.field_u64("regionLength", params.region_length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_register_gpu(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRegisterGpuParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_bool("numaEnabled", params.numa_enabled != 0);
        root.field_i64("numaNodeId", params.numa_node_id as i64);
        root.field_i64("rmCtrlFd", params.rm_ctrl_fd as i64);
        root.field_str("hClient", &hex_u32(params.h_client));
        root.field_str("hSmcPartRef", &hex_u32(params.h_smc_part_ref));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unregister_gpu(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmUnregisterGpuParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_pageable_mem_access(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmPageableMemAccessParams, _>(arg_ptr, root, |params, root| {
        root.field_bool("pageableMemAccess", params.pageable_mem_access != 0);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_prevent_migration_range_groups(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRangeGroupsParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) =
            clamp_count(params.num_group_ids, UVM_MAX_RANGE_GROUPS_PER_IOCTL_CALL);
        root.field_u64("numGroupIds", params.num_group_ids);
        root.field_raw(
            "rangeGroupIds",
            &json_u64_array(&params.range_group_ids[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_allow_migration_range_groups(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmRangeGroupsParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) =
            clamp_count(params.num_group_ids, UVM_MAX_RANGE_GROUPS_PER_IOCTL_CALL);
        root.field_u64("numGroupIds", params.num_group_ids);
        root.field_raw(
            "rangeGroupIds",
            &json_u64_array(&params.range_group_ids[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_set_preferred_location(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmSetPreferredLocationParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_str(
            "preferredLocation",
            &format_uuid(&params.preferred_location),
        );
        root.field_i64(
            "preferredCpuNumaNode",
            params.preferred_cpu_numa_node as i64,
        );
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unset_preferred_location(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmReserveVaParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_enable_read_duplication(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmReserveVaParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_disable_read_duplication(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmReserveVaParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_set_accessed_by(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmSetAccessedByParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_str("accessedByUuid", &format_uuid(&params.accessed_by_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unset_accessed_by(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmSetAccessedByParams, _>(arg_ptr, root, |params, root| {
        root.field_str("requestedBase", &hex_u64(params.requested_base));
        root.field_u64("length", params.length);
        root.field_str("accessedByUuid", &format_uuid(&params.accessed_by_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_migrate(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMigrateParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_str("destinationUuid", &format_uuid(&params.destination_uuid));
        root.field_str("flags", &hex_u32(params.flags));
        root.field_str("semaphoreAddress", &hex_u64(params.semaphore_address));
        root.field_u64("semaphorePayload", params.semaphore_payload as u64);
        root.field_i64("cpuNumaNode", params.cpu_numa_node as i64);
        root.field_str("userSpaceStart", &hex_u64(params.user_space_start));
        root.field_u64("userSpaceLength", params.user_space_length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_migrate_range_group(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMigrateRangeGroupParams, _>(arg_ptr, root, |params, root| {
        root.field_u64("rangeGroupId", params.range_group_id);
        root.field_str("destinationUuid", &format_uuid(&params.destination_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_enable_system_wide_atomics(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmGpuUuidParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_disable_system_wide_atomics(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmGpuUuidParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_init_event_tracker(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsInitEventTrackerParams, _>(arg_ptr, root, |params, root| {
        root.field_str("queueBuffer", &hex_u64(params.queue_buffer));
        root.field_u64("queueBufferSize", params.queue_buffer_size);
        root.field_str("controlBuffer", &hex_u64(params.control_buffer));
        root.field_str("processor", &format_uuid(&params.processor));
        root.field_u64("allProcessors", params.all_processors as u64);
        root.field_u64("uvmFd", params.uvm_fd as u64);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_set_notification_threshold(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsSetNotificationThresholdParams, _>(arg_ptr, root, |params, root| {
        root.field_u64(
            "notificationThreshold",
            params.notification_threshold as u64,
        );
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_event_queue_enable_events(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsFlagsParams, _>(arg_ptr, root, |params, root| {
        root.field_str("eventTypeFlags", &hex_u64(params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_event_queue_disable_events(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsFlagsParams, _>(arg_ptr, root, |params, root| {
        root.field_str("eventTypeFlags", &hex_u64(params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_enable_counters(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsFlagsParams, _>(arg_ptr, root, |params, root| {
        root.field_str("counterTypeFlags", &hex_u64(params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_disable_counters(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsFlagsParams, _>(arg_ptr, root, |params, root| {
        root.field_str("counterTypeFlags", &hex_u64(params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_read_process_memory(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsReadProcessMemoryParams, _>(arg_ptr, root, |params, root| {
        root.field_str("buffer", &hex_u64(params.buffer));
        root.field_u64("size", params.size);
        root.field_str("targetVa", &hex_u64(params.target_va));
        root.field_u64("bytesRead", params.bytes_read);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_write_process_memory(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsWriteProcessMemoryParams, _>(arg_ptr, root, |params, root| {
        root.field_str("buffer", &hex_u64(params.buffer));
        root.field_u64("size", params.size);
        root.field_str("targetVa", &hex_u64(params.target_va));
        root.field_u64("bytesWritten", params.bytes_written);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_get_processor_uuid_table(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmToolsGetProcessorUuidTableParams, _>(arg_ptr, root, |params, root| {
        root.field_str("tablePtr", &hex_u64(params.table_ptr));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_map_dynamic_parallelism_region(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmBaseLengthGpuUuidParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unmap_external(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmBaseLengthGpuUuidParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_tools_flush_events(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmStatusParams, _>(arg_ptr, root, |params, root| {
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_alloc_semaphore_pool(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmAllocSemaphorePoolParams, _>(arg_ptr, root, |params, root| {
        let (count, truncated) = clamp_count(params.gpu_attributes_count, UVM_MAX_GPUS);
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_u64("gpuAttributesCount", params.gpu_attributes_count);
        root.field_raw(
            "perGpuAttributes",
            &json_gpu_mapping_attributes_array(&params.per_gpu_attributes[..count]),
        );
        root.field_bool("truncated", truncated);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_clean_up_zombie_resources(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmStatusParams, _>(arg_ptr, root, |params, root| {
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_pageable_mem_access_on_gpu(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmPageableMemAccessOnGpuParams, _>(arg_ptr, root, |params, root| {
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_bool("pageableMemAccess", params.pageable_mem_access != 0);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_populate_pageable(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmPopulatePageableParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_str("flags", &hex_u32(params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_validate_va_range(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmValidateVaRangeParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_create_external_range(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmCreateExternalRangeParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_map_external_sparse(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmBaseLengthGpuUuidParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_mm_initialize(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmMmInitializeParams, _>(arg_ptr, root, |params, root| {
        root.field_i64("uvmFd", params.uvm_fd as i64);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_alloc_device_p2p(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmAllocDeviceP2pParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_u64("offset", params.offset);
        root.field_str("gpu_uuid", &format_uuid(&params.gpu_uuid));
        root.field_i64("rmCtrlFd", params.rm_ctrl_fd as i64);
        root.field_str("hClient", &hex_u32(params.h_client));
        root.field_str("hMemory", &hex_u32(params.h_memory));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_clear_all_access_counters(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmStatusParams, _>(arg_ptr, root, |params, root| {
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_discard(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmDiscardParams, _>(arg_ptr, root, |params, root| {
        root.field_str("base", &hex_u64(params.base));
        root.field_u64("length", params.length);
        root.field_str("flags", &hex_u64(params.flags));
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_is8_supported(arg_ptr: *mut c_void, root: &mut JsonObject) {
    with_params::<UvmIs8SupportedParams, _>(arg_ptr, root, |params, root| {
        root.field_bool("is8Supported", params.is8_supported != 0);
        root.field_i64("rm_status", params.rm_status as i64);
        root.field_str("status", "ok");
    });
}

fn decode_unknown(arg_ptr: *mut c_void, max_blob: usize, root: &mut JsonObject) {
    let read_len = min(max_blob.max(16), 64);
    if let Some(bytes) = unsafe { read_bytes(arg_ptr as usize, read_len) } {
        root.field_str("blob_hex", &hex_bytes(&bytes));
        root.field_str("status", "unknown_cmd");
    } else {
        root.field_str("status", "arg_unreadable");
    }
}

pub(super) fn uvm_cmd_name(cmd: u64) -> &'static str {
    let cmd = normalize_uvm_cmd(cmd);
    match cmd {
        UVM_INITIALIZE => "UVM_INITIALIZE",
        UVM_DEINITIALIZE => "UVM_DEINITIALIZE",
        UVM_RESERVE_VA => "UVM_RESERVE_VA",
        UVM_RELEASE_VA => "UVM_RELEASE_VA",
        UVM_REGION_COMMIT => "UVM_REGION_COMMIT",
        UVM_REGION_DECOMMIT => "UVM_REGION_DECOMMIT",
        UVM_REGION_SET_STREAM => "UVM_REGION_SET_STREAM",
        UVM_SET_STREAM_RUNNING => "UVM_SET_STREAM_RUNNING",
        UVM_SET_STREAM_STOPPED => "UVM_SET_STREAM_STOPPED",
        UVM_ADD_SESSION => "UVM_ADD_SESSION",
        UVM_ENABLE_COUNTERS => "UVM_ENABLE_COUNTERS",
        UVM_MAP_COUNTER => "UVM_MAP_COUNTER",
        UVM_CREATE_EVENT_QUEUE => "UVM_CREATE_EVENT_QUEUE",
        UVM_REMOVE_EVENT_QUEUE => "UVM_REMOVE_EVENT_QUEUE",
        UVM_MAP_EVENT_QUEUE => "UVM_MAP_EVENT_QUEUE",
        UVM_EVENT_CTRL => "UVM_EVENT_CTRL",
        UVM_GET_GPU_UUID_TABLE => "UVM_GET_GPU_UUID_TABLE",
        UVM_REGION_SET_BACKING => "UVM_REGION_SET_BACKING",
        UVM_REGION_UNSET_BACKING => "UVM_REGION_UNSET_BACKING",
        UVM_CREATE_RANGE_GROUP => "UVM_CREATE_RANGE_GROUP",
        UVM_DESTROY_RANGE_GROUP => "UVM_DESTROY_RANGE_GROUP",
        UVM_REGISTER_GPU_VASPACE => "UVM_REGISTER_GPU_VASPACE",
        UVM_UNREGISTER_GPU_VASPACE => "UVM_UNREGISTER_GPU_VASPACE",
        UVM_REGISTER_CHANNEL => "UVM_REGISTER_CHANNEL",
        UVM_UNREGISTER_CHANNEL => "UVM_UNREGISTER_CHANNEL",
        UVM_ENABLE_PEER_ACCESS => "UVM_ENABLE_PEER_ACCESS",
        UVM_DISABLE_PEER_ACCESS => "UVM_DISABLE_PEER_ACCESS",
        UVM_SET_RANGE_GROUP => "UVM_SET_RANGE_GROUP",
        UVM_MAP_EXTERNAL_ALLOCATION => "UVM_MAP_EXTERNAL_ALLOCATION",
        UVM_FREE => "UVM_FREE",
        UVM_MEM_MAP => "UVM_MEM_MAP",
        UVM_REGISTER_GPU => "UVM_REGISTER_GPU",
        UVM_UNREGISTER_GPU => "UVM_UNREGISTER_GPU",
        UVM_PAGEABLE_MEM_ACCESS => "UVM_PAGEABLE_MEM_ACCESS",
        UVM_PREVENT_MIGRATION_RANGE_GROUPS => "UVM_PREVENT_MIGRATION_RANGE_GROUPS",
        UVM_ALLOW_MIGRATION_RANGE_GROUPS => "UVM_ALLOW_MIGRATION_RANGE_GROUPS",
        UVM_SET_PREFERRED_LOCATION => "UVM_SET_PREFERRED_LOCATION",
        UVM_UNSET_PREFERRED_LOCATION => "UVM_UNSET_PREFERRED_LOCATION",
        UVM_ENABLE_READ_DUPLICATION => "UVM_ENABLE_READ_DUPLICATION",
        UVM_DISABLE_READ_DUPLICATION => "UVM_DISABLE_READ_DUPLICATION",
        UVM_SET_ACCESSED_BY => "UVM_SET_ACCESSED_BY",
        UVM_UNSET_ACCESSED_BY => "UVM_UNSET_ACCESSED_BY",
        UVM_MIGRATE => "UVM_MIGRATE",
        UVM_MIGRATE_RANGE_GROUP => "UVM_MIGRATE_RANGE_GROUP",
        UVM_ENABLE_SYSTEM_WIDE_ATOMICS => "UVM_ENABLE_SYSTEM_WIDE_ATOMICS",
        UVM_DISABLE_SYSTEM_WIDE_ATOMICS => "UVM_DISABLE_SYSTEM_WIDE_ATOMICS",
        UVM_TOOLS_INIT_EVENT_TRACKER => "UVM_TOOLS_INIT_EVENT_TRACKER",
        UVM_TOOLS_SET_NOTIFICATION_THRESHOLD => "UVM_TOOLS_SET_NOTIFICATION_THRESHOLD",
        UVM_TOOLS_EVENT_QUEUE_ENABLE_EVENTS => "UVM_TOOLS_EVENT_QUEUE_ENABLE_EVENTS",
        UVM_TOOLS_EVENT_QUEUE_DISABLE_EVENTS => "UVM_TOOLS_EVENT_QUEUE_DISABLE_EVENTS",
        UVM_TOOLS_ENABLE_COUNTERS => "UVM_TOOLS_ENABLE_COUNTERS",
        UVM_TOOLS_DISABLE_COUNTERS => "UVM_TOOLS_DISABLE_COUNTERS",
        UVM_TOOLS_READ_PROCESS_MEMORY => "UVM_TOOLS_READ_PROCESS_MEMORY",
        UVM_TOOLS_WRITE_PROCESS_MEMORY => "UVM_TOOLS_WRITE_PROCESS_MEMORY",
        UVM_TOOLS_GET_PROCESSOR_UUID_TABLE => "UVM_TOOLS_GET_PROCESSOR_UUID_TABLE",
        UVM_MAP_DYNAMIC_PARALLELISM_REGION => "UVM_MAP_DYNAMIC_PARALLELISM_REGION",
        UVM_UNMAP_EXTERNAL => "UVM_UNMAP_EXTERNAL",
        UVM_TOOLS_FLUSH_EVENTS => "UVM_TOOLS_FLUSH_EVENTS",
        UVM_ALLOC_SEMAPHORE_POOL => "UVM_ALLOC_SEMAPHORE_POOL",
        UVM_CLEAN_UP_ZOMBIE_RESOURCES => "UVM_CLEAN_UP_ZOMBIE_RESOURCES",
        UVM_PAGEABLE_MEM_ACCESS_ON_GPU => "UVM_PAGEABLE_MEM_ACCESS_ON_GPU",
        UVM_POPULATE_PAGEABLE => "UVM_POPULATE_PAGEABLE",
        UVM_VALIDATE_VA_RANGE => "UVM_VALIDATE_VA_RANGE",
        UVM_CREATE_EXTERNAL_RANGE => "UVM_CREATE_EXTERNAL_RANGE",
        UVM_MAP_EXTERNAL_SPARSE => "UVM_MAP_EXTERNAL_SPARSE",
        UVM_MM_INITIALIZE => "UVM_MM_INITIALIZE",
        UVM_TOOLS_INIT_EVENT_TRACKER_V2 => "UVM_TOOLS_INIT_EVENT_TRACKER_V2",
        UVM_TOOLS_GET_PROCESSOR_UUID_TABLE_V2 => "UVM_TOOLS_GET_PROCESSOR_UUID_TABLE_V2",
        UVM_ALLOC_DEVICE_P2P => "UVM_ALLOC_DEVICE_P2P",
        UVM_CLEAR_ALL_ACCESS_COUNTERS => "UVM_CLEAR_ALL_ACCESS_COUNTERS",
        UVM_DISCARD => "UVM_DISCARD",
        UVM_IS_8_SUPPORTED => "UVM_IS_8_SUPPORTED",
        _ => "UNKNOWN",
    }
}

fn normalize_uvm_cmd(cmd: u64) -> u64 {
    cmd & 0xffff_ffff
}

fn clamp_count(count: u64, max: usize) -> (usize, bool) {
    if count > max as u64 {
        (max, true)
    } else {
        (count as usize, false)
    }
}

fn json_u64_array(values: &[u64]) -> String {
    let mut out = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx != 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
    out.push(']');
    out
}

fn json_uuid_array(values: &[[u8; 16]]) -> String {
    let mut out = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx != 0 {
            out.push(',');
        }
        out.push_str(&json_quote(&format_uuid(value)));
    }
    out.push(']');
    out
}

fn json_counter_config_array(values: &[UvmCounterConfig]) -> String {
    let mut out = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx != 0 {
            out.push(',');
        }
        let mut obj = JsonObject::new();
        obj.field_u64("scope", value.scope as u64);
        obj.field_u64("name", value.name as u64);
        obj.field_str("gpu_uuid", &format_uuid(&value.gpu_uuid));
        obj.field_u64("state", value.state as u64);
        out.push_str(&obj.finish());
    }
    out.push(']');
    out
}

fn json_gpu_mapping_attributes_array(values: &[UvmGpuMappingAttributes]) -> String {
    let mut out = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx != 0 {
            out.push(',');
        }
        let mut obj = JsonObject::new();
        obj.field_str("gpu_uuid", &format_uuid(&value.gpu_uuid));
        obj.field_u64("gpuMappingType", value.gpu_mapping_type as u64);
        obj.field_u64("gpuCachingType", value.gpu_caching_type as u64);
        obj.field_u64("gpuFormatType", value.gpu_format_type as u64);
        obj.field_u64("gpuElementBits", value.gpu_element_bits as u64);
        obj.field_u64("gpuCompressionType", value.gpu_compression_type as u64);
        out.push_str(&obj.finish());
    }
    out.push(']');
    out
}

fn hex_u32(value: u32) -> String {
    format!("0x{value:x}")
}

fn hex_u64(value: u64) -> String {
    format!("0x{value:x}")
}

fn format_uuid(uuid: &[u8; 16]) -> String {
    let mut out = String::with_capacity(36);
    for (idx, byte) in uuid.iter().enumerate() {
        if idx == 4 || idx == 6 || idx == 8 || idx == 10 {
            out.push('-');
        }
        out.push(hex_nibble(byte >> 4));
        out.push(hex_nibble(byte & 0x0f));
    }
    out
}

fn hex_nibble(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => '?',
    }
}
