#if __has_feature(cxx_constexpr)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_constexpr)
extension_on
#else
bad_extension
#endif
