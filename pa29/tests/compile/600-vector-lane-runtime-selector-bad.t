typedef int V __attribute__((vector_size(8))); int f(V v,int n){return __builtin_ia32_vec_ext_v2si(v,n);}
