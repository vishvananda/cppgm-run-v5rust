#if __has_include("300-present.h")
present
#endif
#if __has_include("300-missing.h")
bad
#else
absent
#endif
