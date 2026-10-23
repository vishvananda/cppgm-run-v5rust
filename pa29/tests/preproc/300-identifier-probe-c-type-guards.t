#if __is_identifier(_Atomic)
atomic_identifier
#else
bad_atomic_trait
#endif
#if __is_identifier(_BitInt)
bitint_identifier
#else
bad_bitint_trait
#endif
#if !__has_builtin(_Atomic) && !__has_builtin(_BitInt)
c_types_not_builtins
#endif
#if !__is_identifier(__is_class) && !__is_identifier(__remove_cv)
double_underscore_traits
#endif
#if !__is_identifier(int) && !__is_identifier(__inline__)
keywords
#endif
