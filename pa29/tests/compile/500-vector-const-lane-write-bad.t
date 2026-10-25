typedef int words __attribute__((vector_size(16)));
void f(const words& a) { a[0] = 1; }
