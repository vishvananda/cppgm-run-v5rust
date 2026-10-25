typedef signed char lanes __attribute__((vector_size(8)));
lanes f(lanes value, int n) { return value + n; }
