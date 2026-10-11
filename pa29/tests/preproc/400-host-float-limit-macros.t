#if defined(__FLT_MIN__)
have_flt_min
#else
missing_flt_min
#endif

#if defined(__DBL_MIN__)
have_dbl_min
#else
missing_dbl_min
#endif

#if defined(__LDBL_MIN__)
have_ldbl_min
#else
missing_ldbl_min
#endif
