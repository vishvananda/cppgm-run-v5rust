using I = int __attribute__((ext_vector_type(4)));
I f(int value) { return __builtin_convertvector(value, I); }
