pub fn is_gpu_path(path: &str) -> bool {
    path.starts_with("/dev/nvidia") || path.starts_with("/dev/dri/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_prefixes_are_detected() {
        assert!(is_gpu_path("/dev/nvidia0"));
        assert!(is_gpu_path("/dev/nvidia-uvm"));
        assert!(is_gpu_path("/dev/dri/renderD128"));
    }

    #[test]
    fn non_gpu_path_is_filtered() {
        assert!(!is_gpu_path("/tmp/abc"));
    }
}
