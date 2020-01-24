


	.global start
start:

	.extern main
	callq main

	pushq %rax
loop:
	callq exit
	jmp loop



	# void exit(int status)
	.global exit
exit:
	syscall
    jmp exit

	/* memset(void* p, int val, size_t sz) */
	.global memset
memset:
	mov 4(%rsp),%rax	# p
	mov 8(%rsp),%rcx	# val
	mov 12(%rsp),%rdx	# sz

1:
	add $-1,%rdx
	jl 1f
	movb %cl,(%rax,%rdx,1)
	jmp 1b

1:
	ret

	/* memcpy(void* dest, void* src, size_t n) */
	.global memcpy
memcpy:
	mov 4(%rsp),%rax       # dest
        mov 8(%rsp),%rdx       # src
        mov 12(%rsp),%rcx      # n
	push %rbx
1:
	add $-1,%rcx
	jl 1f
	movb (%rdx),%bl
	movb %bl,(%rax)
	add $1,%rdx
	add $1,%rax
	jmp 1b
1:
	pop %rbx
	ret
