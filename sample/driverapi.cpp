// demo.cpp
#include <cuda.h>
#include <cstdio>
#include <cstdlib>
#include <cmath>

static const char* cuGetErrorNameStr(CUresult r) {
    const char* s = nullptr;
    cuGetErrorName(r, &s);
    return s ? s : "UNKNOWN";
}
static const char* cuGetErrorStringStr(CUresult r) {
    const char* s = nullptr;
    cuGetErrorString(r, &s);
    return s ? s : "UNKNOWN";
}

#define CUCHK(call) do { \
    CUresult _r = (call); \
    if (_r != CUDA_SUCCESS) { \
        std::fprintf(stderr, "CUDA Driver error %s (%s) at %s:%d -> %s\n", \
            cuGetErrorNameStr(_r), cuGetErrorStringStr(_r), __FILE__, __LINE__, #call); \
        std::exit(1); \
    } \
} while(0)

int main() {
    // 1) 初始化 Driver API & 创建上下文
    CUCHK(cuInit(0));

    CUdevice dev = 0;
    CUCHK(cuDeviceGet(&dev, 0));

    char devName[256]{};
    CUCHK(cuDeviceGetName(devName, sizeof(devName), dev));
    std::printf("Using GPU: %s\n", devName);

    CUcontext ctx = nullptr;
    CUCHK(cuCtxCreate(&ctx, 0, dev));

    // 2) 准备数据
    const int n = 1 << 20; // 1,048,576
    const size_t bytes = (size_t)n * sizeof(float);

    // cuMemAllocHost: pinned host memory
    float *hA = nullptr, *hB = nullptr, *hC = nullptr;
    CUCHK(cuMemAllocHost((void**)&hA, bytes));
    CUCHK(cuMemAllocHost((void**)&hB, bytes));
    CUCHK(cuMemAllocHost((void**)&hC, bytes));

    for (int i = 0; i < n; ++i) {
        hA[i] = (float)i * 0.5f;
        hB[i] = (float)i * 1.5f;
        hC[i] = 0.0f;
    }

    // 3) cuMemAlloc: device memory
    CUdeviceptr dA = 0, dB = 0, dC = 0;
    CUCHK(cuMemAlloc(&dA, bytes));
    CUCHK(cuMemAlloc(&dB, bytes));
    CUCHK(cuMemAlloc(&dC, bytes));

    // 4) 拷贝 H->D
    CUCHK(cuMemcpyHtoD(dA, hA, bytes));
    CUCHK(cuMemcpyHtoD(dB, hB, bytes));

    // 5) 加载 PTX 并获取 kernel
    CUmodule module = nullptr;
    CUfunction kernel = nullptr;

    CUCHK(cuModuleLoad(&module, "vector_add.ptx"));
    CUCHK(cuModuleGetFunction(&kernel, module, "vecAdd"));

    // 6) 启动 kernel
    const int threads = 256;
    const int blocks = (n + threads - 1) / threads;

    void* args[] = { (void*)&dA, (void*)&dB, (void*)&dC, (void*)&n };

    CUCHK(cuLaunchKernel(
        kernel,
        blocks, 1, 1,      // grid dim
        threads, 1, 1,     // block dim
        0,                 // shared mem bytes
        nullptr,           // stream (nullptr = default stream)
        args,
        nullptr
    ));

    CUCHK(cuCtxSynchronize());

    // 7) 拷贝 D->H
    CUCHK(cuMemcpyDtoH(hC, dC, bytes));

    // 8) 校验
    double maxAbsErr = 0.0;
    for (int i = 0; i < n; ++i) {
        double ref = (double)hA[i] + (double)hB[i];
        double err = std::fabs((double)hC[i] - ref);
        if (err > maxAbsErr) maxAbsErr = err;
    }
    std::printf("Done. maxAbsErr = %.12f\n", maxAbsErr);

    // 9) 释放资源
    CUCHK(cuMemFree(dA));
    CUCHK(cuMemFree(dB));
    CUCHK(cuMemFree(dC));

    CUCHK(cuMemFreeHost(hA));
    CUCHK(cuMemFreeHost(hB));
    CUCHK(cuMemFreeHost(hC));

    CUCHK(cuModuleUnload(module));
    CUCHK(cuCtxDestroy(ctx));

    return 0;
}
