#if __has_feature(cxx_deleted_functions)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_deleted_functions)
extension_on
#else
bad_extension
#endif
