using F = float __attribute__((ext_vector_type(4)));
float f(F value) { return __builtin_reduce_add(value); }
