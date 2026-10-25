typedef signed char lanes __attribute__((vector_size(8)));
lanes f(lanes value) { return value + 256; }
