#if __has_feature(cxx_alignof)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_alignof)
extension_on
#else
bad_extension
#endif
