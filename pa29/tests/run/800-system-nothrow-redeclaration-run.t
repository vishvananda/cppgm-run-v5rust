// N3485 15.4p3 as GCC and Clang apply it to the C library: a function a system
// header declares non-throwing (glibc's posix_memalign) may be redeclared
// without an exception-specification, as clang's mm_malloc.h does; both
// declarations name the library's function
