template<class T> T&& value();
template<class T, class = decltype(value<T>() == value<T>())> char probe(int);
template<class> int probe(...);
static_assert(sizeof(probe<int()>(0)) == sizeof(char), "");
