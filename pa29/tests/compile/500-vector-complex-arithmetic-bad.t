typedef float lanes __attribute__((vector_size(16)));
lanes f(lanes value, __complex__ float other) { return value + other; }
