typedef int V __attribute__((vector_size(8))); constexpr int x=__builtin_ia32_vec_ext_v2si(V{1,2},0);
