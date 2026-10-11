template<class...> struct B {};
template<class... T> using all = B<T...>;
template<class, class> struct H { typedef char type; };
template<> struct H<int, int> { typedef double type; };
template<class T, class U> using A = typename H<T, U>::type;
template<class... S> struct X { template<class T> using fn = all<A<S, T>...>; };
void f(B<double, double> *) {}
int main() { f((X<int, int>::fn<int> *)0); }
