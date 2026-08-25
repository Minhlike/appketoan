#include <windows.h>
#include <stdint.h>
#include <stdlib.h>

#if defined(__x86_64__) || defined(_M_X64)
uintptr_t __security_cookie = 0x2B992DDFA23249D6ULL;
#else
uintptr_t __security_cookie = 0xBB40E64EUL;
#endif

void __security_check_cookie(uintptr_t cookie) {
    (void)cookie;
}

int _Init_thread_epoch = 0;

void _Init_thread_header(int* pOnce) {
    if (!pOnce) return;
    while (1) {
        LONG old = InterlockedCompareExchange((LONG*)pOnce, -1, 0);
        if (old == 0) {
            return;
        }
        if (old == 1) {
            return;
        }
        Sleep(1);
    }
}

void _Init_thread_footer(int* pOnce) {
    if (!pOnce) return;
    InterlockedExchange((LONG*)pOnce, 1);
}

const char nothrow_var[1] __asm__("?nothrow@std@@3Unothrow_t@1@B") = { 0 };

void* operator_new_nothrow(size_t size, const void* nt) __asm__("??2@YAPEAX_KAEBUnothrow_t@std@@@Z");
void* operator_new_nothrow(size_t size, const void* nt) {
    (void)nt;
    return malloc(size);
}

void operator_delete_sized(void* p, size_t size) __asm__("??3@YAXPEAX_K@Z");
void operator_delete_sized(void* p, size_t size) {
    (void)size;
    free(p);
}
