#if __has_feature(cxx_alignas)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_alignas)
extension_on
#else
bad_extension
#endif
