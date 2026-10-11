// N3485 8.5.4p7: 257 does not survive a conversion to __bf16 (8 significant
// bits) and back: narrowing in a list, as Clang and GCC diagnose it.
__bf16 n{257};
