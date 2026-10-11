#if __has_feature(cxx_defaulted_functions)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_defaulted_functions)
extension_on
#else
bad_extension
#endif
