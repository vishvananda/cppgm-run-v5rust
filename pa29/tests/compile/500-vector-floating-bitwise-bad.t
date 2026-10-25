typedef float floats __attribute__((vector_size(16)));
floats f(floats value) { return ~value; }
