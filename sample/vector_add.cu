// vector_add.cu
extern "C" __global__
void vecAdd(const float* a, const float* b, float* c, int n) {
    int i = (int)(blockIdx.x * blockDim.x + threadIdx.x);
    if (i < n) c[i] = a[i] + b[i];
}
