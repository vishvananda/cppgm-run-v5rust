#if __has_feature(__cxx_binary_literals__)
feature_on
#else
bad_feature
#endif
#if __has_extension(__cxx_binary_literals__)
extension_on
#else
bad_extension
#endif
