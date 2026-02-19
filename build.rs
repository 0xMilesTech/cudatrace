use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod build_specs_embed;

const DRIVER_MANUAL_HOOKS_FILE: &str = "src/hook/driver.rs";
const CUDART_MANUAL_HOOKS_FILE: &str = "src/hook/cudart.rs";
const GENERATED_NVIDIA_IOCTL_TABLES_FILE: &str = "generated_nvidia_ioctl_tables.rs";

const CUDA_ERROR_UNKNOWN: i32 = 999;

#[derive(Debug, Clone)]
struct WrapperPrototype {
    decl_prefix: String,
    name: String,
    params: String,
    arg_names: Vec<String>,
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_generated_driver_wrappers)");
    println!("cargo:rustc-check-cfg=cfg(has_generated_cudart_wrappers)");
    println!("cargo:rerun-if-changed={DRIVER_MANUAL_HOOKS_FILE}");
    println!("cargo:rerun-if-changed={CUDART_MANUAL_HOOKS_FILE}");
    println!("cargo:rerun-if-env-changed=CUDA_ROOT");
    println!("cargo:rerun-if-env-changed=CUDA_INCLUDE");

    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").expect("OUT_DIR is always set by cargo when running build.rs"),
    );

    generate_nvidia_ioctl_tables(&out_dir);

    let Some(cuda_include) = resolve_cuda_include() else {
        println!("cargo:warning=CUDA include path not found; skipping generated CUDA wrappers");
        return;
    };
    if !cuda_include.join("cuda.h").is_file() {
        println!(
            "cargo:warning=CUDA header missing at {}; skipping generated CUDA wrappers",
            cuda_include.join("cuda.h").display()
        );
        return;
    }
    if !cuda_include.join("cuda_runtime_api.h").is_file() {
        println!(
            "cargo:warning=CUDA runtime header missing at {}; skipping generated CUDA wrappers",
            cuda_include.join("cuda_runtime_api.h").display()
        );
        return;
    }

    let mut generated_any = false;
    if generate_driver_wrappers(&cuda_include, &out_dir) {
        println!("cargo:rustc-cfg=has_generated_driver_wrappers");
        generated_any = true;
    }
    if generate_cudart_wrappers(&cuda_include, &out_dir) {
        println!("cargo:rustc-cfg=has_generated_cudart_wrappers");
        generated_any = true;
    }

    if generated_any {
        let exports_map = out_dir.join("cudatrace_exports.map");
        if let Err(err) = fs::write(&exports_map, render_exports_version_script()) {
            println!(
                "cargo:warning=failed to write {} ({err}); generated wrappers may not be exported",
                exports_map.display()
            );
        } else {
            println!(
                "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
                exports_map.display()
            );
        }
    }
}

fn generate_driver_wrappers(cuda_include: &Path, out_dir: &Path) -> bool {
    let proto_text = build_specs_embed::DRIVER_MISSING_PROTOTYPES;
    let Some(manual_text) = read_optional_file(DRIVER_MANUAL_HOOKS_FILE) else {
        println!(
            "cargo:warning=missing {}; skipping generated driver wrappers",
            DRIVER_MANUAL_HOOKS_FILE
        );
        return false;
    };

    let manual_hooks = parse_manual_hook_names(&manual_text, "cu");
    let wrappers = parse_missing_prototypes(proto_text, &manual_hooks, "cu");
    if wrappers.is_empty() {
        println!("cargo:warning=no generated driver wrappers produced");
        return false;
    }

    let generated_c = out_dir.join("driver_generated_wrappers.c");
    let source = render_driver_c_source(&wrappers);
    if let Err(err) = fs::write(&generated_c, source) {
        println!(
            "cargo:warning=failed to write {} ({err}); skipping generated driver wrappers",
            generated_c.display()
        );
        return false;
    }

    cc::Build::new()
        .file(&generated_c)
        .include(cuda_include)
        .flag_if_supported("-Wno-deprecated-declarations")
        .flag_if_supported("-Wno-unused-parameter")
        .compile("cudatrace_driver_generated");

    true
}

fn generate_cudart_wrappers(_cuda_include: &Path, _out_dir: &Path) -> bool {
    println!(
        "cargo:warning=generated runtime wrappers are disabled; using manual cudart hooks only"
    );
    false
}

fn resolve_cuda_include() -> Option<PathBuf> {
    if let Some(path) = env::var_os("CUDA_INCLUDE") {
        let p = PathBuf::from(path);
        if p.join("cuda.h").is_file() {
            return Some(p);
        }
    }

    if let Some(path) = env::var_os("CUDA_ROOT") {
        let p = PathBuf::from(path).join("include");
        if p.join("cuda.h").is_file() {
            return Some(p);
        }
    }

    let default = Path::new("/usr/local/cuda/include");
    if default.join("cuda.h").is_file() {
        return Some(default.to_path_buf());
    }

    None
}

fn read_optional_file(path: &str) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(v) => Some(v),
        Err(err) => {
            println!("cargo:warning=failed to read {path} ({err})");
            None
        }
    }
}

fn generate_nvidia_ioctl_tables(out_dir: &Path) {
    // Keep build self-contained: do not read external open-gpu/spec snapshots.
    let out = r#"// @generated by build.rs; do not edit manually.
pub const GENERATED_RM_CONTROL_CMD_NAMES: &[(u32, &str)] = &[
    (0x13eu32, "NV0000_CTRL_CMD_SYSTEM_GET_BUILD_VERSION_V2"),
    (0x20800147u32, "NV2080_CTRL_CMD_GPU_GET_ENGINE_PARTNERLIST"),
    (0x20800601u32, "NV2080_CTRL_CMD_I2C_READ_BUFFER"),
];
pub const GENERATED_NV_ESC_CMD_NAMES: &[(u32, &str)] = &[];
pub const GENERATED_RM_INTERFACE_NAMES: &[(u32, &str)] = &[
    (0x208101u32, "NV2081_BINAPI"),
];
"#;

    let output_path = out_dir.join(GENERATED_NVIDIA_IOCTL_TABLES_FILE);
    if let Err(err) = fs::write(&output_path, out) {
        println!(
            "cargo:warning=failed to write {} ({err})",
            output_path.display()
        );
    }
}

fn parse_manual_hook_names(text: &str, symbol_prefix: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("pub unsafe extern \"C\" fn ") else {
            continue;
        };
        let Some(name) = rest.split('(').next() else {
            continue;
        };
        let name = name.trim();
        if name.starts_with(symbol_prefix) {
            out.insert(name.to_string());
        }
    }
    out
}

fn parse_missing_prototypes(
    text: &str,
    manual_hooks: &HashSet<String>,
    symbol_prefix: &str,
) -> Vec<WrapperPrototype> {
    let mut wrappers = Vec::new();
    let mut seen = HashSet::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let line = line.trim_end_matches(';');

        let Some(open_paren) = line.find('(') else {
            continue;
        };
        let Some(close_paren) = line.rfind(')') else {
            continue;
        };
        if close_paren <= open_paren {
            continue;
        }

        let before_paren = line[..open_paren].trim();
        let Some(name) = before_paren.split_whitespace().last() else {
            continue;
        };
        if !name.starts_with(symbol_prefix) {
            continue;
        }

        let Some(name_pos) = before_paren.rfind(name) else {
            continue;
        };
        let decl_prefix = before_paren[..name_pos].trim();
        if decl_prefix.is_empty() {
            continue;
        }
        if manual_hooks.contains(name) {
            continue;
        }
        if !seen.insert(name.to_string()) {
            continue;
        }

        let params_raw = line[open_paren + 1..close_paren].trim();
        let params = sanitize_params(params_raw);
        let Ok(arg_names) = parse_param_names(&params) else {
            println!(
                "cargo:warning=skip generated wrapper for {name}: failed to parse parameter names"
            );
            continue;
        };

        wrappers.push(WrapperPrototype {
            decl_prefix: decl_prefix.to_string(),
            name: name.to_string(),
            params,
            arg_names,
        });
    }

    wrappers.sort_by(|a, b| a.name.cmp(&b.name));
    wrappers
}

fn sanitize_params(params: &str) -> String {
    if params.trim().is_empty() || params.trim() == "void" {
        return String::new();
    }

    let mut out = Vec::new();
    for raw in params.split(',') {
        let mut p = raw.trim().to_string();
        if p.is_empty() || p == "void" {
            continue;
        }

        p = remove_macro_invocation(&p, "__dv");
        if let Some(eq_pos) = p.find('=') {
            p = p[..eq_pos].trim().to_string();
        }
        p = collapse_spaces(&p);
        if !p.is_empty() && p != "void" {
            out.push(p);
        }
    }

    out.join(", ")
}

fn remove_macro_invocation(input: &str, macro_name: &str) -> String {
    let mut s = input.to_string();
    let needle = format!("{macro_name}(");

    while let Some(start) = s.find(&needle) {
        let mut i = start + needle.len();
        let bytes = s.as_bytes();
        let mut depth = 1usize;
        while i < bytes.len() {
            match bytes[i] as char {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        if depth != 0 || i >= bytes.len() {
            break;
        }

        s.replace_range(start..=i, "");
    }

    s
}

fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_param_names(params: &str) -> Result<Vec<String>, ()> {
    if params.trim().is_empty() || params.trim() == "void" {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for raw in params.split(',') {
        let param = raw.trim();
        if param.is_empty() || param == "void" {
            continue;
        }
        let name = extract_param_name(param).ok_or(())?;
        names.push(name.to_string());
    }
    Ok(names)
}

fn extract_param_name(param: &str) -> Option<&str> {
    let bytes = param.as_bytes();
    let mut end = bytes.len();

    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let mut start = end;
    while start > 0 {
        let c = bytes[start - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            start -= 1;
        } else {
            break;
        }
    }

    if start == end {
        return None;
    }

    Some(&param[start..end])
}

fn render_driver_c_source(wrappers: &[WrapperPrototype]) -> String {
    let mut out = String::new();
    out.push_str("#include <cuda.h>\n");
    out.push_str("#include <stddef.h>\n");
    out.push_str("#include <string.h>\n\n");
    out.push_str("extern void* cudatrace_driver_enter(const char* func_name);\n");
    out.push_str("extern void cudatrace_driver_exit(void* scope, CUresult result);\n");
    out.push_str("extern void* cudatrace_resolve_driver_symbol(const char* symbol);\n\n");
    out.push_str(&format!(
        "#define CUDATRACE_CUDA_ERROR_UNKNOWN ((CUresult){CUDA_ERROR_UNKNOWN})\n\n"
    ));

    for wrapper in wrappers {
        out.push_str(&format!(
            "#ifdef {name}\n#undef {name}\n#endif\n",
            name = wrapper.name
        ));
    }
    out.push('\n');

    for wrapper in wrappers {
        let params = if wrapper.params.is_empty() {
            "void".to_string()
        } else {
            wrapper.params.clone()
        };
        let call_args = wrapper.arg_names.join(", ");

        out.push_str(&format!(
            "{decl} {name}({params}) {{\n",
            decl = wrapper.decl_prefix,
            name = wrapper.name,
            params = params
        ));
        out.push_str(&format!(
            "    void* cudatrace_scope_token = cudatrace_driver_enter(\"{name}\");\n",
            name = wrapper.name
        ));
        out.push_str(&format!(
            "    typedef {decl} (*real_fn_t)({params});\n",
            decl = wrapper.decl_prefix,
            params = params
        ));
        out.push_str("    static real_fn_t real_fn = NULL;\n");
        out.push_str("    if (!real_fn) {\n");
        out.push_str(&format!(
            "        real_fn = (real_fn_t)cudatrace_resolve_driver_symbol(\"{name}\");\n",
            name = wrapper.name
        ));
        out.push_str("    }\n");
        if wrapper.arg_names.is_empty() {
            out.push_str(
                "    CUresult ret = real_fn ? real_fn() : CUDATRACE_CUDA_ERROR_UNKNOWN;\n",
            );
        } else {
            out.push_str(&format!(
                "    CUresult ret = real_fn ? real_fn({call_args}) : CUDATRACE_CUDA_ERROR_UNKNOWN;\n"
            ));
        }
        out.push_str("    cudatrace_driver_exit(cudatrace_scope_token, ret);\n");
        out.push_str("    return ret;\n");
        out.push_str("}\n\n");
    }

    out.push_str("typedef struct {\n");
    out.push_str("    const char* name;\n");
    out.push_str("    void* fn;\n");
    out.push_str("} cudatrace_driver_wrapper_entry_t;\n\n");

    out.push_str("static const cudatrace_driver_wrapper_entry_t kGeneratedDriverWrappers[] = {\n");
    for wrapper in wrappers {
        out.push_str(&format!(
            "    {{\"{name}\", (void*){name}}},\n",
            name = wrapper.name
        ));
    }
    out.push_str("};\n\n");

    out.push_str("void* cudatrace_lookup_generated_driver_wrapper(const char* symbol) {\n");
    out.push_str("    if (!symbol) {\n");
    out.push_str("        return NULL;\n");
    out.push_str("    }\n");
    out.push_str(
        "    for (size_t i = 0; i < sizeof(kGeneratedDriverWrappers) / sizeof(kGeneratedDriverWrappers[0]); ++i) {\n",
    );
    out.push_str("        if (strcmp(symbol, kGeneratedDriverWrappers[i].name) == 0) {\n");
    out.push_str("            return kGeneratedDriverWrappers[i].fn;\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("    return NULL;\n");
    out.push_str("}\n");

    out
}

#[allow(dead_code)]
fn render_cudart_c_source(wrappers: &[WrapperPrototype]) -> String {
    let mut out = String::new();
    out.push_str("#include <cuda_runtime_api.h>\n");
    out.push_str("#include <stddef.h>\n");
    out.push_str("#include <string.h>\n\n");
    out.push_str("extern void* cudatrace_cudart_enter(const char* func_name);\n");
    out.push_str("extern void cudatrace_cudart_exit(void* scope, cudaError_t result);\n");
    out.push_str("extern void* cudatrace_resolve_cudart_symbol(const char* symbol);\n\n");
    out.push_str(&format!(
        "#define CUDATRACE_CUDART_ERROR_UNKNOWN ((cudaError_t){CUDA_ERROR_UNKNOWN})\n\n"
    ));

    for wrapper in wrappers {
        out.push_str(&format!(
            "#ifdef {name}\n#undef {name}\n#endif\n",
            name = wrapper.name
        ));
    }
    out.push('\n');

    for wrapper in wrappers {
        let params = if wrapper.params.is_empty() {
            "void".to_string()
        } else {
            wrapper.params.clone()
        };
        let call_args = wrapper.arg_names.join(", ");

        out.push_str(&format!(
            "{decl} {name}({params}) {{\n",
            decl = wrapper.decl_prefix,
            name = wrapper.name,
            params = params
        ));
        out.push_str(&format!(
            "    void* cudatrace_scope_token = cudatrace_cudart_enter(\"{name}\");\n",
            name = wrapper.name
        ));
        out.push_str(&format!(
            "    typedef {decl} (*real_fn_t)({params});\n",
            decl = wrapper.decl_prefix,
            params = params
        ));
        out.push_str("    static real_fn_t real_fn = NULL;\n");
        out.push_str("    if (!real_fn) {\n");
        out.push_str(&format!(
            "        real_fn = (real_fn_t)cudatrace_resolve_cudart_symbol(\"{name}\");\n",
            name = wrapper.name
        ));
        out.push_str("    }\n");
        if wrapper.arg_names.is_empty() {
            out.push_str(
                "    cudaError_t ret = real_fn ? real_fn() : CUDATRACE_CUDART_ERROR_UNKNOWN;\n",
            );
        } else {
            out.push_str(&format!(
                "    cudaError_t ret = real_fn ? real_fn({call_args}) : CUDATRACE_CUDART_ERROR_UNKNOWN;\n"
            ));
        }
        out.push_str("    cudatrace_cudart_exit(cudatrace_scope_token, ret);\n");
        out.push_str("    return ret;\n");
        out.push_str("}\n\n");
    }

    out.push_str("typedef struct {\n");
    out.push_str("    const char* name;\n");
    out.push_str("    void* fn;\n");
    out.push_str("} cudatrace_cudart_wrapper_entry_t;\n\n");

    out.push_str("static const cudatrace_cudart_wrapper_entry_t kGeneratedCudartWrappers[] = {\n");
    for wrapper in wrappers {
        out.push_str(&format!(
            "    {{\"{name}\", (void*){name}}},\n",
            name = wrapper.name
        ));
    }
    out.push_str("};\n\n");

    out.push_str("void* cudatrace_lookup_generated_cudart_wrapper(const char* symbol) {\n");
    out.push_str("    if (!symbol) {\n");
    out.push_str("        return NULL;\n");
    out.push_str("    }\n");
    out.push_str(
        "    for (size_t i = 0; i < sizeof(kGeneratedCudartWrappers) / sizeof(kGeneratedCudartWrappers[0]); ++i) {\n",
    );
    out.push_str("        if (strcmp(symbol, kGeneratedCudartWrappers[i].name) == 0) {\n");
    out.push_str("            return kGeneratedCudartWrappers[i].fn;\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("    return NULL;\n");
    out.push_str("}\n");

    out
}

fn render_exports_version_script() -> &'static str {
    r#"{
  global:
    cu*;
    cuda*;
    cudatrace*;
    dlsym;
  local:
    *;
};
"#
}
