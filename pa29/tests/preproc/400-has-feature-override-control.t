#if __has_feature(cxx_override_control)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_override_control)
extension_on
#else
bad_extension
#endif
