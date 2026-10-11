// VALIDATION: compile-pass
// Partial ordering must count placeholders nested in a template-id.
template<class T, int N> struct array {};
template<class> struct trait;
template<class T, int N> struct trait<array<array<T, 4>, N> > { enum { value = 1 }; };
template<class T> struct trait<array<array<T, 4>, 4> > { enum { value = 2 }; };
int main() { return trait<array<array<int, 4>, 4> >::value != 2; }
