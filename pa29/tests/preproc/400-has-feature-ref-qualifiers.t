#if __has_feature(cxx_reference_qualified_functions)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_reference_qualified_functions)
extension_on
#else
bad_extension
#endif
#if __cpp_ref_qualifiers >= 200710L
cpp_feature_on
#else
bad_cpp_macro
#endif
