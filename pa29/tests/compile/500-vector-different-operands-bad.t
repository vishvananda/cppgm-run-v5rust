typedef int words __attribute__((vector_size(16)));
typedef float floats __attribute__((vector_size(16)));
auto f(words a, floats b) -> decltype(a == b);
