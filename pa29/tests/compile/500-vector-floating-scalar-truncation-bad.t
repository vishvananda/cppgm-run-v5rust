typedef float lanes __attribute__((vector_size(16)));
lanes f(lanes value, double n) { return value + n; }
