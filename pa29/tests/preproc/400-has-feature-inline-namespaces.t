#if __has_feature(cxx_inline_namespaces)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_inline_namespaces)
extension_on
#else
bad_extension
#endif
