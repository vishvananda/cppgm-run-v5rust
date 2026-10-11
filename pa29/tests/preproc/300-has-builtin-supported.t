#if __has_builtin(__remove_reference_t) && __has_builtin(__is_trivially_destructible) && __has_builtin(__reference_constructs_from_temporary) && __has_builtin(__reference_binds_to_temporary)
good
#else
bad
#endif
