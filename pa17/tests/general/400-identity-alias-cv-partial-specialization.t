template<class T, class...> using A = T;
template<class> struct X;
template<class T> struct X<A<T const, int>> : X<T> {};
template<> struct X<int> { static const int value = 1; };
static_assert(X<int const>::value == 1, "");
int main() {}
