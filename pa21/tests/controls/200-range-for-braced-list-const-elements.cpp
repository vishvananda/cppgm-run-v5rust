// N3485 8.5.4p5, 18.9p1, 6.5.4p1: a braced-init-list range iterates
// std::initializer_list<E>'s array of const E: its begin() is a const E*, so
// `*__begin` is a const E lvalue: `auto&` and `auto&&` deduce const E&,
// overload resolution and member function calls see a const object, and an
// `auto` copy is modifiable. std::initializer_list is declared (8.5.4p2), not
// defined; expect exit 0
namespace std { template<class T> class initializer_list; }
int f(int&) { return 1; }
int f(const int&) { return 2; }
struct V { int n; V(int a) : n(a) {} int g() { return 10; } int g() const { return 20; } };
template<class T> struct is_const_ref { static const int v = 0; };
template<class T> struct is_const_ref<const T&> { static const int v = 1; };
int main() {
  int s = 0;
  for (auto& x : {1, 2, 3}) s += f(x);
  for (auto&& y : {4, 5}) s += f(y) + is_const_ref<decltype(y)>::v * 100;
  for (const auto& z : {6}) s += f(z);
  for (auto w : {7, 8}) { w = 1; s += f(w) + w; }
  for (auto& v : {V(1), V(2)}) s += v.g();
  for (V v : {V(1), V(2)}) s += v.g();
  for (auto& x : {1, 2}) s += is_const_ref<decltype(x)>::v * 1000;
  return s == 2276 ? 0 : 1;
}
