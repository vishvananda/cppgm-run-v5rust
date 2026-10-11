struct A { virtual void f() = 0; };
template<class T, class = decltype(T())> char value(int);
template<class> long value(...);
template<class T, class = decltype(::new T)> char allocate(int);
template<class> long allocate(...);
static_assert(sizeof(value<A>(0)) == sizeof(long), "");
static_assert(sizeof(allocate<A>(0)) == sizeof(long), "");
