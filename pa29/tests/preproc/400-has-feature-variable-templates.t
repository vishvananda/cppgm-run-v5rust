#if __has_feature(__cxx_variable_templates__)
feature_on
#else
bad_feature
#endif
#if __has_extension(__cxx_variable_templates__)
extension_on
#else
bad_extension
#endif
