#if __has_builtin(__is_signed)
signed_known
#else
bad_signed
#endif
#if __has_builtin(__is_same)
same_known
#else
bad_same
#endif
#if __has_builtin(__remove_reference_t)
transform_known
#else
bad_transform
#endif
#if __has_builtin(__is_never_going_to_exist)
bad_unknown
#else
unknown_off
#endif
