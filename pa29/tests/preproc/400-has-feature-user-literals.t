#if __has_feature(cxx_user_literals)
feature_on
#else
bad_feature
#endif
#if __has_extension(cxx_user_literals)
extension_on
#else
bad_extension
#endif
