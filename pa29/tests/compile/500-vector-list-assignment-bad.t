typedef int words __attribute__((vector_size(16)));
void f(words& value) { value = {1,2}; }
