using B = bool __attribute__((ext_vector_type(4)));
bool f(B value) { return __builtin_reduce_max(value); }
