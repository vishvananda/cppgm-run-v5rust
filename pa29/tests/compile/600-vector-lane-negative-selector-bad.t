typedef int V __attribute__((vector_size(8))); int f(V v){return __builtin_ia32_vec_ext_v2si(v,-1);}
