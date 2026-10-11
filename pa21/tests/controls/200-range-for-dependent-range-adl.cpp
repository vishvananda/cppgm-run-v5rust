// N3485 6.5.4p1, 14.6.2.2p3, 14.6.4.2: in an instantiation, range-based for
// over a type-dependent range is type-dependent: `__begin != __end`,
// `++__begin`, `*__begin`, begin(r) and end(r), and calls taking an `auto`
// loop variable, find functions declared after the template by
// argument-dependent lookup from the point of instantiation, also over a
// braced-init-list of dependent elements; expect exit 0
namespace std { template<class T> class initializer_list; }
namespace N {
struct S { int v; };
struct R { S d[2]; S* begin() { return d; } S* end() { return d + 2; } };
struct It { int* p; };
struct Q { int d[2]; It begin() { It i = {d}; return i; } It end() { It i = {d + 2}; return i; } };
struct P { int d[2]; };
}
template<class T> int f(T t) { int s = 0; for (auto x : t) s += g(x); for (auto& y : t.d) s += g(y) + (y + 1); return s; }
template<class T> int h(T t) { int s = 0; for (const auto& y : {t, t}) s += g(y); return s; }
template<class T> int k(T t) { int s = 0; for (int x : t) s += x; return s; }
namespace N {
int g(S s) { return s.v; }
int operator+(S s, int k) { return s.v * k; }
bool operator!=(It a, It b) { return a.p != b.p; }
It& operator++(It& a) { ++a.p; return a; }
int& operator*(It a) { return *a.p; }
int* begin(P& p) { return p.d; }
int* end(P& p) { return p.d + 2; }
}
int main() {
  N::R r = {{{1}, {2}}};
  N::S s = {3};
  N::Q q = {{4, 5}};
  N::P p = {{6, 7}};
  return f(r) == 9 && h(s) == 6 && k(q) == 9 && k(p) == 13 ? 0 : 1;
}
