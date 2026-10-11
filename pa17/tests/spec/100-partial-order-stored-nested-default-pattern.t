// VALIDATION: compile-pass
// N3485 focus: 14.5.5.2 [temp.class.order]
template<class C, class T = int> struct sw {};
template<class T, long N> struct box {};
template<class B, class C> struct pick { enum { value = 0 }; };
template<class T, long N, class C, class U>
struct pick<box<T, N>, sw<C, U> > { enum { value = 1 }; };
template<class T, long N, class C>
struct pick<box<T, N>, sw<C> > { enum { value = 2 }; };
static_assert(pick<box<int, 2>, sw<char> >::value == 2, "");
int main() {}
