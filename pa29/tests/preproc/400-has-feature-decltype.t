#if __has_feature(cxx_decltype)
decltype_feature_on
#else
bad_decltype_feature
#endif
#if __has_extension(cxx_decltype)
decltype_extension_on
#else
bad_decltype_extension
#endif
#if __has_feature(cxx_decltype_incomplete_return_types)
n3276_feature_on
#else
bad_n3276_feature
#endif
#if __has_extension(cxx_decltype_incomplete_return_types)
n3276_extension_on
#else
bad_n3276_extension
#endif
