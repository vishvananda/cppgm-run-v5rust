#if __has_cpp_attribute(no_unique_address)
present
#else
absent
#endif
#if __has_cpp_attribute(__no_unique_address__)
spelled
#endif
#if __has_cpp_attribute(imaginary_attribute) || __has_cpp_attribute(gnu::imaginary)
unexpected
#else
unknown
#endif
