// Deduction-guide declarations are not C++11 syntax. PA29 accepts them in its
// GNU++11 compatibility lane because hosted STL headers contain this syntax.
template<class T, class U>
struct pair_like {};

template<class T, class U>
pair_like(T, U) -> pair_like<T, U>;
