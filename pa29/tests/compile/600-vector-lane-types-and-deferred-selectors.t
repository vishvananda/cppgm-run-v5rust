typedef int I __attribute__((vector_size(8)));
typedef short S __attribute__((vector_size(8)));
#if !__has_builtin(__builtin_ia32_vec_init_v2si)
#error missing vector builtin
#endif
static_assert(__is_same(decltype(__builtin_ia32_vec_init_v2si(1,2)), I), "init");
static_assert(__is_same(decltype(__builtin_ia32_vec_ext_v4hi(S{},3)), short), "extract");
static_assert(__is_same(decltype(__builtin_ia32_vec_set_v4hi(S{},3,2)), S), "insert");
I source() noexcept;
I throwing();
static_assert(noexcept(__builtin_ia32_vec_ext_v2si(source(),0)), "nonthrowing operation");
static_assert(noexcept(__builtin_ia32_vec_ext_v2si(throwing(),0)), "frozen call-intrinsic query");
template<class T> int deferred(T x) { return __builtin_ia32_vec_ext_v2si(x,9); }
template<class T> int unknown(T x,int n) { return __builtin_ia32_vec_ext_v2si(x,n); }
template<class T> auto query(T x) -> decltype(__builtin_ia32_vec_ext_v2si(x,0));
char query(...);
static_assert(__is_same(decltype(query(I{})), int), "valid substitution");
static_assert(__is_same(decltype(query(3)), char), "failed substitution");
int f(I v) { return ::__builtin_ia32_vec_ext_v2si(v,1); }
