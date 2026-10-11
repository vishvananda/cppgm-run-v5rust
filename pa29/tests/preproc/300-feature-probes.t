#if __has_feature(modules)
bad
#else
modules_off
#endif
#if __has_extension(blocks)
bad
#else
blocks_off
#endif
#if __has_attribute(always_inline)
attr_on
#else
bad
#endif
#if __has_cpp_attribute(nodiscard)
bad
#else
cppattr_off
#endif
#if __building_module(_Builtin_stddef)
bad
#else
building_module_off
#endif
