#if __has_cpp_attribute(_Clang::__no_destroy__)
bad_scoped
#else
scoped_off
#endif
#if __has_cpp_attribute(msvc::no_unique_address)
bad_vendor
#else
vendor_off
#endif
#if __has_cpp_attribute(__nodiscard__)
bad_plain
#else
plain_off
#endif
