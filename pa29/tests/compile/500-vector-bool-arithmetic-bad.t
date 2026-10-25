using flags __attribute__((ext_vector_type(17))) = bool;
flags f(flags a, flags b) { a += b; return a; }
