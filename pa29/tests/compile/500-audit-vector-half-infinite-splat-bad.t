typedef _Float16 halves __attribute__((vector_size(8)));
halves add(halves value) { return value + __builtin_huge_val(); }
