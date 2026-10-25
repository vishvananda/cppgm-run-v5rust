typedef short V __attribute__((vector_size(8))); template<class T> __attribute__((always_inline)) inline V f(V v,T n){return __builtin_ia32_vec_set_v4hi(v,7,n);} V g(V v,int n){return f(v,n);}
