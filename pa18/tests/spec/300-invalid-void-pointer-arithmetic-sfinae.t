template<class T> T&& value();
template<class T, class U, class = decltype(value<T>() + value<U>())> char probe(int);
template<class, class> int probe(...);
static_assert(sizeof(probe<bool, void*>(0)) == sizeof(int), "");
