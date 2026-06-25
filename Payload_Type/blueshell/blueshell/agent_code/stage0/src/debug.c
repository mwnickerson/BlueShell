#include "stage0.h"

#ifdef STAGE0_DEBUG
#include <stdio.h>

void s0_debug(const wchar_t *message) {
    fwprintf(stderr, L"[stage0] %ls\n", message);
    fflush(stderr);
}

void s0_debug_error(const wchar_t *message, DWORD error) {
    fwprintf(stderr, L"[stage0] %ls (error=%lu)\n", message,
             (unsigned long)error);
    fflush(stderr);
}
#else
void s0_debug(const wchar_t *message) { (void)message; }
void s0_debug_error(const wchar_t *message, DWORD error) {
    (void)message;
    (void)error;
}
#endif
