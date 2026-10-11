#if __has_feature(cxx_auto_type)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_auto_type)
extension_on
#else
bad_extension
#endif
