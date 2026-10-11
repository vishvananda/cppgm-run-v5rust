#if __has_feature(cxx_rvalue_references)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_rvalue_references)
extension_on
#else
bad_extension
#endif
#if __cpp_rvalue_references >= 200610L
cpp_feature_on
#else
bad_cpp_macro
#endif
