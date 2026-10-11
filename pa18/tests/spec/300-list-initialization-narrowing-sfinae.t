struct X { X(int); };
template<class T, class = decltype(T{0.5})> char probe(int);
template<class> long probe(...);
static_assert(sizeof(probe<int>(0)) == sizeof(long), "");
static_assert(sizeof(probe<X>(0)) == sizeof(long), "");
