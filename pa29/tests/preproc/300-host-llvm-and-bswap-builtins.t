#if defined(__clang__) && !defined(__llvm__)
missing_llvm
#elif __has_builtin(__builtin_bswap16) && __has_builtin(__builtin_bswap32) && __has_builtin(__builtin_bswap64)
good
#else
bad
#endif
