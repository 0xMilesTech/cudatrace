// vector_add_pinned_host.cu
#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <cuda_runtime.h>
#include <unistd.h>   // close
#include <fcntl.h>    // open

#define CUDA_CHECK(call) do {                                   \
  cudaError_t err = (call);                                     \
  if (err != cudaSuccess) {                                     \
    fprintf(stderr, "CUDA error %s:%d: %s\n",                   \
            __FILE__, __LINE__, cudaGetErrorString(err));       \
    std::exit(1);                                               \
  }                                                             \
} while (0)

__global__ void vecAdd(const float* A, const float* B, float* C, int N) {
  int i = blockIdx.x * blockDim.x + threadIdx.x;
  if (i < N) C[i] = A[i] + B[i];
}

int main() {
  // open() system call at program start
  int fd = open("/dev/null", O_RDONLY);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  close(fd);

  const int N = 1 << 20;
  const size_t bytes = N * sizeof(float);
  // 1) Pinned host allocations (cudaMallocHost)
  float *hA = nullptr, *hB = nullptr, *hC = nullptr;
  CUDA_CHECK(cudaMallocHost(&hA, bytes));
  CUDA_CHECK(cudaMallocHost(&hB, bytes));
  CUDA_CHECK(cudaMallocHost(&hC, bytes));

  for (int i = 0; i < N; i++) {
    hA[i] = (float)(i % 100) * 0.5f;
    hB[i] = (float)(i % 100) * 0.25f;
  }

  // Device allocations
  float *dA = nullptr, *dB = nullptr, *dC = nullptr;
  CUDA_CHECK(cudaMalloc(&dA, bytes));
  CUDA_CHECK(cudaMalloc(&dB, bytes));
  CUDA_CHECK(cudaMalloc(&dC, bytes));

  // Stream for async copies + kernel
  cudaStream_t stream;
  CUDA_CHECK(cudaStreamCreate(&stream));

  // H->D async (works best with pinned host memory)
  CUDA_CHECK(cudaMemcpyAsync(dA, hA, bytes, cudaMemcpyHostToDevice, stream));
  CUDA_CHECK(cudaMemcpyAsync(dB, hB, bytes, cudaMemcpyHostToDevice, stream));

  const int threads = 256;
  const int blocks = (N + threads - 1) / threads;

  // Timing (events recorded on the same stream)
  cudaEvent_t start, stop;
  CUDA_CHECK(cudaEventCreate(&start));
  CUDA_CHECK(cudaEventCreate(&stop));

  CUDA_CHECK(cudaEventRecord(start, stream));
  vecAdd<<<blocks, threads, 0, stream>>>(dA, dB, dC, N);
  CUDA_CHECK(cudaGetLastError());
  CUDA_CHECK(cudaMemcpyAsync(hC, dC, bytes, cudaMemcpyDeviceToHost, stream));
  CUDA_CHECK(cudaEventRecord(stop, stream));
  CUDA_CHECK(cudaEventSynchronize(stop));

  float ms = 0.0f;
  CUDA_CHECK(cudaEventElapsedTime(&ms, start, stop));

  // Verify (after stream work done, hC is ready)
  int bad = 0;
  for (int i = 0; i < N; i++) {
    float ref = hA[i] + hB[i];
    if (fabsf(hC[i] - ref) > 1e-6f) {
      bad++;
      if (bad < 5) printf("Mismatch i=%d: %f vs %f\n", i, hC[i], ref);
    }
  }

  printf("Pinned-host vecAdd N=%d, blocks=%d, threads=%d\n", N, blocks, threads);
  printf("End-to-end (H2D + kernel + D2H) time: %.3f ms\n", ms);
  printf("Verify: %s (bad=%d)\n", bad == 0 ? "PASS" : "FAIL", bad);

  // Cleanup
  CUDA_CHECK(cudaEventDestroy(start));
  CUDA_CHECK(cudaEventDestroy(stop));
  CUDA_CHECK(cudaStreamDestroy(stream));
  CUDA_CHECK(cudaFree(dA));
  CUDA_CHECK(cudaFree(dB));
  CUDA_CHECK(cudaFree(dC));
  CUDA_CHECK(cudaFreeHost(hA));
  CUDA_CHECK(cudaFreeHost(hB));
  CUDA_CHECK(cudaFreeHost(hC));

  CUDA_CHECK(cudaDeviceReset());
  return bad == 0 ? 0 : 2;
}
