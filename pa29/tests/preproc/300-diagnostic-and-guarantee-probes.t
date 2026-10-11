#if __has_warning("-Wunused-variable")
bad_warning
#else
warning_off
#endif
#if __has_declspec_attribute(dllexport)
bad_declspec
#else
declspec_off
#endif
#if __has_constexpr_builtin(__builtin_fabs)
bad_constexpr_builtin
#else
constexpr_builtin_off
#endif
#if __has_rmw_builtin(__atomic_fetch_add)
bad_rmw
#else
rmw_off
#endif
