use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const TRACEFS_ROOTS: [&str; 2] = ["/sys/kernel/tracing", "/sys/kernel/debug/tracing"];

pub struct TraceFsController {
    root: PathBuf,
}

pub fn preflight() -> Result<(), String> {
    let controller = TraceFsController::init_or_fail()?;
    controller.reset_state()
}

impl TraceFsController {
    pub fn init_or_fail() -> Result<Self, String> {
        let Some(root) = detect_tracefs_root() else {
            return Err("tracefs is not available under /sys/kernel/tracing or /sys/kernel/debug/tracing".to_owned());
        };

        let this = Self { root };
        this.validate_access()?;
        this.apply_runtime_config()?;
        Ok(this)
    }

    pub fn begin_capture(&self, tid: i32, graph_function: Option<&str>) -> Result<(), String> {
        self.write_control("tracing_on", "0")?;
        self.write_control("set_ftrace_pid", "")?;
        self.write_control("set_ftrace_pid", &tid.to_string())?;
        self.best_effort_set_graph_function(graph_function);
        self.write_control("current_tracer", "function_graph")?;
        self.write_control("trace", "")?;
        self.write_control("tracing_on", "1")
    }

    pub fn end_capture_and_read(&self) -> Result<String, String> {
        self.write_control("tracing_on", "0")?;
        let trace = fs::read_to_string(self.path("trace"))
            .map_err(|err| format!("failed to read trace: {err}"))?;
        self.write_control("trace", "")?;
        self.best_effort_set_graph_function(None);
        self.write_control("set_ftrace_pid", "")?;
        Ok(trace)
    }

    pub fn reset_state(&self) -> Result<(), String> {
        self.write_control("tracing_on", "0")?;
        self.best_effort_set_graph_function(None);
        self.write_control("set_ftrace_pid", "")?;
        self.write_control("current_tracer", "nop")?;
        self.write_control("trace", "")
    }

    fn validate_access(&self) -> Result<(), String> {
        for name in [
            "available_tracers",
            "current_tracer",
            "set_ftrace_pid",
            "trace",
            "tracing_on",
            "buffer_size_kb",
        ] {
            let path = self.path(name);
            if !path.exists() {
                return Err(format!("required tracefs file missing: {}", path.display()));
            }
        }

        let available = fs::read_to_string(self.path("available_tracers"))
            .map_err(|err| format!("failed to read available_tracers: {err}"))?;
        if !available.split_whitespace().any(|token| token == "function_graph") {
            return Err("kernel does not expose function_graph tracer".to_owned());
        }

        // Validate write permission up front so failures are reported before runtime.
        self.write_control("tracing_on", "0")?;
        self.write_control("set_ftrace_pid", "")?;
        self.write_control("current_tracer", "nop")?;
        self.write_control("trace", "")
    }

    fn apply_runtime_config(&self) -> Result<(), String> {
        let buffer_kb = crate::config::global().fgraph_buffer_kb;
        self.write_control("buffer_size_kb", &buffer_kb.to_string())
            .map_err(|err| format!("failed to set buffer_size_kb={buffer_kb}: {err}"))?;
        Ok(())
    }

    fn write_control(&self, file: &str, value: &str) -> Result<(), String> {
        let path = self.path(file);
        let mut handle = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|err| format!("failed to open {}: {err}", path.display()))?;

        // tracefs control files generally expect a single write operation.
        // For `trace` clear, a single newline keeps shell behavior parity (`echo > trace`).
        let payload: &[u8] = if value.is_empty() {
            b"\n"
        } else {
            value.as_bytes()
        };
        handle
            .write_all(payload)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))
    }

    fn best_effort_set_graph_function(&self, graph_function: Option<&str>) {
        if !self.path("set_graph_function").exists() {
            return;
        }
        let _ = self.write_control("set_graph_function", "");
        if let Some(graph_function) = graph_function {
            if self
                .write_control("set_graph_function", graph_function)
                .is_err()
            {
                let _ = self.write_control("set_graph_function", "");
            }
        }
    }

    fn path(&self, file: &str) -> PathBuf {
        self.root.join(file)
    }
}

impl Drop for TraceFsController {
    fn drop(&mut self) {
        let _ = self.reset_state();
    }
}

fn detect_tracefs_root() -> Option<PathBuf> {
    TRACEFS_ROOTS
        .iter()
        .map(Path::new)
        .find(|path| path.is_dir())
        .map(Path::to_path_buf)
}
