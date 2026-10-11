// N3485 6.5.4p1, 7.1.6.4p6, 14.6.2.2: in a template definition, an `auto` loop
// variable over a braced-init-list of type-dependent elements has a dependent
// type until the instantiation deduces it; non-dependent elements give the
// variable const E (8.5.4p5), and an `auto&&` element of a non-dependent array
// is an lvalue reference (14.8.2.1p3); expect exit 0
namespace std { template<class T> class initializer_list; }
template<class T> struct is_const_ref { static const int v = 0; };
template<class T> struct is_const_ref<const T&> { static const int v = 1; };
template<class T> struct lref { static const int v = 0; };
template<class T> struct lref<T&> { static const int v = 5; };
int h(int&) { return 1; }
int h(const int&) { return 2; }
template<class T> int f(T a) {
  int s = 0;
  int k = 3;
  for (auto x : {a, a}) s += x;
  for (const auto& y : {1, 2}) s += y;
  for (auto& z : {k, k}) s += h(z) * 10;
  for (auto&& w : {k, a}) s += is_const_ref<decltype(w)>::v * 100;
  for (auto v : {T(4)}) { v = 1; s += v; }
  int arr[2] = {1, 2};
  for (auto&& e : arr) s += lref<decltype(e)>::v * 1000;
  T tarr[2] = {1, 2};
  for (auto&& e : tarr) s += lref<decltype(e)>::v * 1000 + h(e);
  for (const auto* p : {&k, &k}) s += *p;
  for (auto g : {[](int x) { return x + 1; }}) s += g(a);
  return s;
}
template<class T> struct C { T m; int g() { int s = 0; for (auto& x : {m, m}) s += h(x); return s; } };
int main() { C<int> c = {1}; return f(3) + f(2) * 7 == 162075 && c.g() == 4 ? 0 : 1; }
