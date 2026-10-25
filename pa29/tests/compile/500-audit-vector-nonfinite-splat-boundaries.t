typedef _Float16 halves __attribute__((vector_size(8)));
typedef float singles __attribute__((vector_size(16)));
halves nan(halves value) { return value + __builtin_nan(""); }
singles infinite(singles value) { return value + __builtin_huge_val(); }
halves finite(halves value) { return value + 65504.0; }
