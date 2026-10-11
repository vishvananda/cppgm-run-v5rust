#if __has_feature(cxx_trailing_return)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_trailing_return)
extension_on
#else
bad_extension
#endif
