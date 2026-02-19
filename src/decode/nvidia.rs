pub const NV_IOCTL_MAGIC: u8 = b'F';
pub const NV_IOCTL_BASE: u32 = 200;

// nv-ioctl-numbers.h
pub const NV_ESC_CARD_INFO: u32 = NV_IOCTL_BASE;
pub const NV_ESC_REGISTER_FD: u32 = NV_IOCTL_BASE + 1;
pub const NV_ESC_ALLOC_OS_EVENT: u32 = NV_IOCTL_BASE + 6;
pub const NV_ESC_FREE_OS_EVENT: u32 = NV_IOCTL_BASE + 7;
pub const NV_ESC_STATUS_CODE: u32 = NV_IOCTL_BASE + 9;
pub const NV_ESC_CHECK_VERSION_STR: u32 = NV_IOCTL_BASE + 10;
pub const NV_ESC_IOCTL_XFER_CMD: u32 = NV_IOCTL_BASE + 11;
pub const NV_ESC_ATTACH_GPUS_TO_FD: u32 = NV_IOCTL_BASE + 12;
pub const NV_ESC_QUERY_DEVICE_INTR: u32 = NV_IOCTL_BASE + 13;
pub const NV_ESC_SYS_PARAMS: u32 = NV_IOCTL_BASE + 14;
pub const NV_ESC_NUMA_INFO: u32 = NV_IOCTL_BASE + 15;
pub const NV_ESC_SET_NUMA_STATUS: u32 = NV_IOCTL_BASE + 16;
pub const NV_ESC_EXPORT_TO_DMABUF_FD: u32 = NV_IOCTL_BASE + 17;
pub const NV_ESC_WAIT_OPEN_COMPLETE: u32 = NV_IOCTL_BASE + 18;

// nv_escape.h (legacy RM escapes used heavily by libcuda)
pub const NV_ESC_RM_ALLOC_MEMORY: u32 = 0x27;
pub const NV_ESC_RM_ALLOC_OBJECT: u32 = 0x28;
pub const NV_ESC_RM_FREE: u32 = 0x29;
pub const NV_ESC_RM_CONTROL: u32 = 0x2a;
pub const NV_ESC_RM_ALLOC: u32 = 0x2b;
pub const NV_ESC_RM_CONFIG_GET: u32 = 0x32;
pub const NV_ESC_RM_CONFIG_SET: u32 = 0x33;
pub const NV_ESC_RM_DUP_OBJECT: u32 = 0x34;
pub const NV_ESC_RM_SHARE: u32 = 0x35;
pub const NV_ESC_RM_CONFIG_GET_EX: u32 = 0x37;
pub const NV_ESC_RM_CONFIG_SET_EX: u32 = 0x38;
pub const NV_ESC_RM_I2C_ACCESS: u32 = 0x39;
pub const NV_ESC_RM_IDLE_CHANNELS: u32 = 0x41;
pub const NV_ESC_RM_VID_HEAP_CONTROL: u32 = 0x4a;
pub const NV_ESC_RM_ACCESS_REGISTRY: u32 = 0x4d;
pub const NV_ESC_RM_MAP_MEMORY: u32 = 0x4e;
pub const NV_ESC_RM_UNMAP_MEMORY: u32 = 0x4f;
pub const NV_ESC_RM_GET_EVENT_DATA: u32 = 0x52;
pub const NV_ESC_RM_ALLOC_CONTEXT_DMA2: u32 = 0x54;
pub const NV_ESC_RM_ADD_VBLANK_CALLBACK: u32 = 0x56;
pub const NV_ESC_RM_MAP_MEMORY_DMA: u32 = 0x57;
pub const NV_ESC_RM_UNMAP_MEMORY_DMA: u32 = 0x58;
pub const NV_ESC_RM_BIND_CONTEXT_DMA: u32 = 0x59;
pub const NV_ESC_RM_EXPORT_OBJECT_TO_FD: u32 = 0x5c;
pub const NV_ESC_RM_IMPORT_OBJECT_FROM_FD: u32 = 0x5d;
pub const NV_ESC_RM_UPDATE_DEVICE_MAPPING_INFO: u32 = 0x5e;
pub const NV_ESC_RM_LOCKLESS_DIAGNOSTIC: u32 = 0x5f;

pub const NV04_CONTROL: u32 = 0x00000036;

pub const NV01_ROOT: i32 = 0x00000000;
pub const NV01_ROOT_NON_PRIV: i32 = 0x00000001;
pub const NV01_ROOT_CLIENT: i32 = 0x00000041;

pub const NV0000_CTRL_CMD_SYSTEM_GET_BUILD_VERSION: u32 = 0x0101;
pub const NV0000_CTRL_CMD_CLIENT_GET_ADDR_SPACE_TYPE: u32 = 0x0d01;
pub const NV0000_CTRL_CMD_OS_UNIX_GET_CONTROL_FILE_DESCRIPTOR: u32 = 0x3d04;

pub const NV0000_CTRL_CLIENT_GET_ADDR_SPACE_TYPE_INVALID: u32 = 0x0000_0000;
pub const NV0000_CTRL_CLIENT_GET_ADDR_SPACE_TYPE_SYSMEM: u32 = 0x0000_0001;
pub const NV0000_CTRL_CLIENT_GET_ADDR_SPACE_TYPE_VIDMEM: u32 = 0x0000_0002;
pub const NV0000_CTRL_CLIENT_GET_ADDR_SPACE_TYPE_REGMEM: u32 = 0x0000_0003;
pub const NV0000_CTRL_CLIENT_GET_ADDR_SPACE_TYPE_FABRIC: u32 = 0x0000_0004;
pub const NV0000_CTRL_CLIENT_GET_ADDR_SPACE_TYPE_FABRIC_MC: u32 = 0x0000_0005;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvPciInfo {
    pub domain: u32,
    pub bus: u8,
    pub slot: u8,
    pub function: u8,
    pub _pad0: u8,
    pub vendor_id: u16,
    pub device_id: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlCardInfo {
    pub valid: u8,
    pub _pad0: [u8; 3],
    pub pci_info: NvPciInfo,
    pub gpu_id: u32,
    pub interrupt_line: u16,
    pub _pad1: [u8; 2],
    pub reg_address: u64,
    pub reg_size: u64,
    pub fb_address: u64,
    pub fb_size: u64,
    pub minor_number: u32,
    pub dev_name: [u8; 10],
    pub _pad2: [u8; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlRegisterFd {
    pub ctl_fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlAllocOsEvent {
    pub h_client: u32,
    pub h_device: u32,
    pub fd: u32,
    pub status: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlStatusCode {
    pub domain: u32,
    pub bus: u8,
    pub slot: u8,
    pub _pad0: u16,
    pub status: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvIoctlRmApiVersion {
    pub cmd: u32,
    pub reply: u32,
    pub version_string: [u8; 64],
}

impl Default for NvIoctlRmApiVersion {
    fn default() -> Self {
        Self {
            cmd: 0,
            reply: 0,
            version_string: [0; 64],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlQueryDeviceIntr {
    pub intr_status: u32,
    pub status: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlSysParams {
    pub memblock_size: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlWaitOpenComplete {
    pub rc: i32,
    pub adapter_status: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NvOfflineAddresses {
    pub addresses: [u64; 64],
    pub num_entries: u32,
}

impl Default for NvOfflineAddresses {
    fn default() -> Self {
        Self {
            addresses: [0; 64],
            num_entries: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlNumaInfo {
    pub nid: i32,
    pub status: i32,
    pub memblock_size: u64,
    pub numa_mem_addr: u64,
    pub numa_mem_size: u64,
    pub use_auto_online: u8,
    pub _pad0: [u8; 7],
    pub offline_addresses: NvOfflineAddresses,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlSetNumaStatus {
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlExportToDmaBufFdPrefix {
    pub fd: i32,
    pub h_client: u32,
    pub total_objects: u32,
    pub num_objects: u32,
    pub index: u32,
    pub total_size: u64,
    pub mapping_type: u8,
    pub b_allow_mmap: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlXfer {
    pub cmd: u32,
    pub size: u32,
    pub ptr: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs00Parameters {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_old: u32,
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs02Parameters {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_new: u32,
    pub h_class: i32,
    pub flags: u32,
    pub p_memory: u64,
    pub limit: u64,
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlNvos02ParametersWithFd {
    pub params: NvOs02Parameters,
    pub fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs05Parameters {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_new: u32,
    pub h_class: i32,
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs21Parameters {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_new: u32,
    pub h_class: i32,
    pub p_alloc_parms: u64,
    pub params_size: u32,
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs64Parameters {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_new: u32,
    pub h_class: i32,
    pub p_alloc_parms: u64,
    pub p_rights_requested: u64,
    pub params_size: u32,
    pub flags: u32,
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs33Parameters {
    pub h_client: u32,
    pub h_device: u32,
    pub h_memory: u32,
    pub offset: u64,
    pub length: u64,
    pub p_linear_address: u64,
    pub status: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvIoctlNvos33ParametersWithFd {
    pub params: NvOs33Parameters,
    pub fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs34Parameters {
    pub h_client: u32,
    pub h_device: u32,
    pub h_memory: u32,
    pub p_linear_address: u64,
    pub status: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NvOs54Parameters {
    pub h_client: u32,
    pub h_object: u32,
    pub cmd: u32,
    pub flags: u32,
    pub params: u64,
    pub params_size: u32,
    pub status: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Nv0000CtrlClientGetAddrSpaceTypeParams {
    pub h_object: u32,
    pub map_flags: u32,
    pub addr_space_type: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Nv0000CtrlOsUnixGetControlFileDescriptorParams {
    pub fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Nv0000CtrlSystemGetBuildVersionParams {
    pub size_of_strings: u32,
    pub p_driver_version_buffer: u64,
    pub p_version_buffer: u64,
    pub p_title_buffer: u64,
    pub changelist_number: u32,
    pub official_changelist_number: u32,
}

#[cfg(test)]
mod tests {
    use super::{
        Nv0000CtrlClientGetAddrSpaceTypeParams, Nv0000CtrlOsUnixGetControlFileDescriptorParams,
        Nv0000CtrlSystemGetBuildVersionParams, NvIoctlAllocOsEvent, NvIoctlCardInfo,
        NvIoctlExportToDmaBufFdPrefix, NvIoctlNumaInfo, NvIoctlNvos02ParametersWithFd,
        NvIoctlNvos33ParametersWithFd, NvIoctlQueryDeviceIntr, NvIoctlRegisterFd,
        NvIoctlRmApiVersion, NvIoctlSetNumaStatus, NvIoctlStatusCode, NvIoctlSysParams,
        NvIoctlWaitOpenComplete, NvIoctlXfer, NvOs00Parameters, NvOs02Parameters, NvOs05Parameters,
        NvOs21Parameters, NvOs33Parameters, NvOs34Parameters, NvOs54Parameters, NvOs64Parameters,
        NvPciInfo,
    };

    #[test]
    fn nvidia_struct_layout_matches_headers() {
        assert_eq!(std::mem::size_of::<NvPciInfo>(), 12);
        assert_eq!(std::mem::size_of::<NvIoctlCardInfo>(), 72);

        assert_eq!(std::mem::size_of::<NvIoctlXfer>(), 16);
        assert_eq!(std::mem::align_of::<NvIoctlXfer>(), 8);

        assert_eq!(std::mem::size_of::<NvIoctlRegisterFd>(), 4);
        assert_eq!(std::mem::size_of::<NvIoctlAllocOsEvent>(), 16);
        assert_eq!(std::mem::size_of::<NvIoctlStatusCode>(), 12);
        assert_eq!(std::mem::size_of::<NvIoctlRmApiVersion>(), 72);
        assert_eq!(std::mem::size_of::<NvIoctlQueryDeviceIntr>(), 8);
        assert_eq!(std::mem::size_of::<NvIoctlSysParams>(), 8);
        assert_eq!(std::mem::size_of::<NvIoctlWaitOpenComplete>(), 8);
        assert_eq!(std::mem::size_of::<NvIoctlNumaInfo>(), 560);
        assert_eq!(std::mem::size_of::<NvIoctlSetNumaStatus>(), 4);
        assert_eq!(std::mem::size_of::<NvIoctlExportToDmaBufFdPrefix>(), 40);

        assert_eq!(std::mem::size_of::<NvOs00Parameters>(), 16);
        assert_eq!(std::mem::size_of::<NvOs02Parameters>(), 48);
        assert_eq!(std::mem::size_of::<NvIoctlNvos02ParametersWithFd>(), 56);
        assert_eq!(std::mem::size_of::<NvOs05Parameters>(), 20);
        assert_eq!(std::mem::size_of::<NvOs21Parameters>(), 32);
        assert_eq!(std::mem::size_of::<NvOs64Parameters>(), 48);
        assert_eq!(std::mem::size_of::<NvOs33Parameters>(), 48);
        assert_eq!(std::mem::size_of::<NvIoctlNvos33ParametersWithFd>(), 56);
        assert_eq!(std::mem::size_of::<NvOs34Parameters>(), 32);

        assert_eq!(std::mem::size_of::<NvOs54Parameters>(), 32);
        assert_eq!(std::mem::align_of::<NvOs54Parameters>(), 8);

        assert_eq!(
            std::mem::size_of::<Nv0000CtrlClientGetAddrSpaceTypeParams>(),
            12
        );
        assert_eq!(
            std::mem::align_of::<Nv0000CtrlClientGetAddrSpaceTypeParams>(),
            4
        );

        assert_eq!(
            std::mem::size_of::<Nv0000CtrlOsUnixGetControlFileDescriptorParams>(),
            4
        );
        assert_eq!(
            std::mem::align_of::<Nv0000CtrlOsUnixGetControlFileDescriptorParams>(),
            4
        );

        assert_eq!(
            std::mem::size_of::<Nv0000CtrlSystemGetBuildVersionParams>(),
            40
        );
        assert_eq!(
            std::mem::align_of::<Nv0000CtrlSystemGetBuildVersionParams>(),
            8
        );
    }
}
