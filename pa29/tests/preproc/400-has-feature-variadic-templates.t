#if __has_feature(cxx_variadic_templates)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_variadic_templates)
extension_on
#else
bad_extension
#endif
