template<class T> T &&val();
template<class T> void take(T);
template<class F, class T> auto probe(int) -> decltype(take<T>(val<F>()), char());
template<class, class> int probe(...);
struct A { virtual void f() = 0; };
static_assert(sizeof(probe<A &, A>(0)) == sizeof(int), "");
int main() { return 0; }
