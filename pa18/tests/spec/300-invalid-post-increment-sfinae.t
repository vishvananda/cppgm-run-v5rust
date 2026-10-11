template<class T> T&& value();
template<class T, class = decltype(value<T>()++)> char probe(int);
template<class> int probe(...);
static_assert(sizeof(probe<const int&>(0)) == sizeof(int), "");
static_assert(sizeof(probe<void*&>(0)) == sizeof(int), "");
