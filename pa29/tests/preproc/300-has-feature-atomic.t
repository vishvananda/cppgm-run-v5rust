#if __has_feature(cxx_atomic) || __has_extension(c_atomic)
good
#else
bad
#endif
