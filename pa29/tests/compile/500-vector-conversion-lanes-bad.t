using I = int __attribute__((ext_vector_type(4)));
using F = float __attribute__((ext_vector_type(2)));
F f(I value) { return __builtin_convertvector(value, F); }
