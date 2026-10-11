struct M {};
struct C {};
template<class A, class B> struct Pair {};
template<class T> struct Wrap { Wrap(T); };
namespace N { namespace B {
template<class T> M pick(T &);
template<class T> C pick(const T &);
} using namespace B;
template<class A, class B> Pair<A, B> pair(const A &, const B &);
}
template<class... T>
auto f(T &&...x) -> Wrap<decltype(N::pair(N::pick(x)...))> {
  return Wrap<decltype(N::pair(N::pick(x)...))>(N::pair(N::pick(x)...));
}
void g(const int &x, int &y) { (void)sizeof(f(x, x)); f(x, y); }
int main() { return 0; }
