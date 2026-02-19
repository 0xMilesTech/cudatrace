# cudatrace

`cudatrace` is a Rust `cdylib` (`libcudatrace.so`) for `LD_PRELOAD` tracing of:

- CUDA Runtime API (`cudart`)
- CUDA Driver API (`libcuda`)
- Linux syscalls (focus on `ioctl` and GPU file descriptors)

Trace output defaults to per-thread files:

- `./cudatrace.output.tid-<tid>`

## Build

```bash
cargo build
```

The shared library is generated at:

```bash
target/debug/libcudatrace.so
```

## Quick Run (sample/cudart)

```bash
nvcc ./sample/cudart.cu -o ./sample/cudart -cudart=shared
LD_PRELOAD=$PWD/target/debug/libcudatrace.so ./sample/cudart
```

Default output files:

```bash
ls ./cudatrace.output.tid-*
```

## Quick Run (sample/driverapi)

```bash
nvcc -ptx sample/vector_add.cu -o sample/vector_add.ptx
g++ -O2 sample/driverapi.cpp -o sample/driverapi -lcuda
cd sample
LD_PRELOAD=../target/debug/libcudatrace.so ./driverapi
```

## Environment Variables

- `LIB_CUDATRACE_OUTPUT`:
  - `file` (default)
  - `stdout`
- `LIB_CUDATRACE_PATH`:
  - default `./cudatrace.output`
  - `file` mode writes `LIB_CUDATRACE_PATH.tid-<tid>`
- `LIB_CUDATRACE_TRACE`:
  - `all` (default)
  - comma list: `cudart,driver,syscall`
- `LIB_CUDATRACE_IOCTL_DECODE`:
  - `full` (default)
  - `header`
  - `off`
- `LIB_CUDATRACE_MAX_BLOB`:
  - default `256`
- `LIB_CUDATRACE_TIME_UNIT`:
  - `us` (default)
  - `ns`
- `LIB_CUDATRACE_LEFT_META`:
  - `off` (default)
  - `tid`
  - `ts` / `timestamp`
  - `tid,ts`
  - when enabled, output lines get a left prefix like `tid=12345 ts=1739950000000000us  |  ...`
- `LIB_CUDATRACE_PTRACE_DEBUG`:
  - optional debug logs (`1/true/yes/on`)

Example:

```bash
LIB_CUDATRACE_OUTPUT=stdout \
LIB_CUDATRACE_TRACE=driver,syscall \
LIB_CUDATRACE_IOCTL_DECODE=header \
LD_PRELOAD=$PWD/target/debug/libcudatrace.so \
./sample/driverapi
```

Notes for ptrace syscall tracing:

- `LIB_CUDATRACE_TRACE` contains `syscall` means enabling ptrace syscall capture.
- `ptrace` capture is fused into the same output stream/files.
- Syscall trace output is produced by the built-in `ptrace` path.
- Output format follows syscall line style: `name(args) = ret  /* time */`.
- Requires kernel ptrace permission (Yama/LSM settings may affect availability).
