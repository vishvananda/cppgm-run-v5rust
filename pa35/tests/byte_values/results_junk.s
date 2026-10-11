	.text
	.globl _Z5ext_bl
_Z5ext_bl:
	movabs $0x123456789abcde00, %rax
	cmpq $3, %rdi
	setg %al
	ret
	.globl _Z5ext_ul
_Z5ext_ul:
	imul $7, %rdi, %rdx
	movabs $0x7654321000000000, %rax
	movb %dl, %al
	ret
	.globl _Z5ext_sl
_Z5ext_sl:
	imul $13, %rdi, %rdx
	movabs $0x0fedcba900000000, %rax
	movb %dl, %al
	ret
	.section .note.GNU-stack,"",@progbits
