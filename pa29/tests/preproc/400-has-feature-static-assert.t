#if __has_feature(cxx_static_assert)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_static_assert)
extension_on
#else
bad_extension
#endif
#if __cpp_static_assert >= 200410L
cpp_feature_on
#else
bad_cpp_macro
#endif
