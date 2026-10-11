/* te(En) for an abi-tagged enumeration En (GCC abi_tag "e"): _Z2te2EnB1e. */
#if defined(__APPLE__)
#define CPPGM_HOST_SYMBOL(name) "_" name
#else
#define CPPGM_HOST_SYMBOL(name) name
#endif
int te_definition(int e) __asm__(CPPGM_HOST_SYMBOL("_Z2te2EnB1e"));
int te_definition(int e) { return e; }
