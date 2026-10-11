#if defined(__EXCEPTIONS)
#error exceptions_macro_still_defined
#endif

#if defined(__cpp_exceptions)
#error cpp_exceptions_still_defined
#endif

#if __has_feature(cxx_exceptions)
#error feature_exceptions_still_true
#endif

#if __has_extension(cxx_exceptions)
#error extension_exceptions_still_true
#endif

int main() {}
