typedef int lanes __attribute__((vector_size(16)));
lanes f(lanes value, int* other) { return value + other; }
