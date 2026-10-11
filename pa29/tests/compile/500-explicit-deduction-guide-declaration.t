// Deduction-guide declarations are not C++11 syntax. PA29 accepts them in its
// GNU++11 compatibility lane because hosted STL headers contain this syntax.
template<class T>
struct wrap {};

template<class T>
explicit wrap(T) -> wrap<T>;
