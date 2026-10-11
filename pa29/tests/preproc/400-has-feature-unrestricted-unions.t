#if __has_feature(cxx_unrestricted_unions)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_unrestricted_unions)
extension_on
#else
bad_extension
#endif
