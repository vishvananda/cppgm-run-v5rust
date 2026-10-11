// N3485 14.6p8, p10: a non-dependent placement new-expression in a template is
// resolved where the template is defined: no allocation function declared
// there is viable (the operator new declared after the template does not make
// the definition valid).
#include <cstddef>
template<class T> void* make() { return new (5) int; }
void* operator new(std::size_t n, int) { return ::operator new(n); }
void* use() { return make<char>(); }
int main() { return 0; }
