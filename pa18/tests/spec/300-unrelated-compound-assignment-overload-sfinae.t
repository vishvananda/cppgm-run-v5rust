struct A {};
bool operator+=(const A&, const A&);
struct X {};
template<class T> T&& value();
template<class T, class U, class = decltype(value<T>() += value<U>())> char probe(int);
template<class, class> int probe(...);
static_assert(sizeof(probe<X&, X&>(0)) == sizeof(int), "");
static_assert(sizeof(probe<bool&, int*>(0)) == sizeof(char), "");
