    .text
    .globl  __security_check_cookie
__security_check_cookie:
    ret

    .globl  _Init_thread_header
_Init_thread_header:
    testq   %rcx, %rcx
    jz      .Lhead_ret
.Lhead_loop:
    movl    $0, %eax
    movl    $-1, %edx
    lock cmpxchgl %edx, (%rcx)
    testl   %eax, %eax
    jz      .Lhead_ret
    cmpl    $1, %eax
    je      .Lhead_ret
    subq    $40, %rsp
    movl    $1, %ecx
    call    Sleep
    addq    $40, %rsp
    jmp     .Lhead_loop
.Lhead_ret:
    ret

    .globl  _Init_thread_footer
_Init_thread_footer:
    testq   %rcx, %rcx
    jz      .Lfoot_ret
    movl    $1, (%rcx)
.Lfoot_ret:
    ret

    .globl  "??2@YAPEAX_KAEBUnothrow_t@std@@@Z"
"??2@YAPEAX_KAEBUnothrow_t@std@@@Z":
    subq    $40, %rsp
    call    malloc
    addq    $40, %rsp
    ret

    .globl  "??3@YAXPEAX_K@Z"
"??3@YAXPEAX_K@Z":
    subq    $40, %rsp
    call    free
    addq    $40, %rsp
    ret

    .data
    .globl  __security_cookie
    .align  8
__security_cookie:
    .quad   0x2B992DDFA23249D6

    .globl  _Init_thread_epoch
    .align  4
_Init_thread_epoch:
    .long   0

    .globl  "?nothrow@std@@3Unothrow_t@1@B"
"?nothrow@std@@3Unothrow_t@1@B":
    .byte   0