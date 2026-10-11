#if !defined(__EXCEPTIONS)
#error no_exceptions_macro
#endif

#if !defined(__cpp_exceptions)
#error no_cpp_exceptions
#endif

#if !defined(__GXX_RTTI)
#error no_gxx_rtti
#endif

#if !defined(__cpp_rtti)
#error no_cpp_rtti
#endif

#if !__has_feature(cxx_exceptions)
#error no_feature_exceptions
#endif

#if !__has_feature(cxx_rtti)
#error no_feature_rtti
#endif

int main() {}
