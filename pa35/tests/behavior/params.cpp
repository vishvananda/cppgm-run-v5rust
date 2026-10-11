// By-value object parameters copied into locals, changed, looped over,
// escaping and returned (layout_slots: a parameter's home in its slot).
struct V { unsigned char kind; unsigned index; };
struct P { long a; long b; };
struct T { int x, y, z; };
struct A { V* alias; };
__attribute__((noinline)) void touch(V* v) { v->index += 1; }
__attribute__((noinline)) V resolve(const A& a, V v) { while (v.kind == 1 && a.alias[v.index].kind != 0) v = a.alias[v.index]; return v; }
__attribute__((noinline)) long sum(P p) { long s = 0; for (int i = 0; i < 3; ++i) { s += p.a; p.a = p.b; p.b = s; } return s + p.a; }
__attribute__((noinline)) T bump(T t, int k) { t.y += k; if (k & 1) t.z = t.x; return t; }
__attribute__((noinline)) V esc(V v) { touch(&v); return v; }
__attribute__((noinline)) long first(P p, P q) { if (p.a < q.a) p = q; return p.a * 3 + p.b; }
#include <cstdio>
int main() {
  V tab[8]; for (int i = 0; i < 8; ++i) { tab[i].kind = (i % 3 == 0) ? 0 : 1; tab[i].index = (i * 5 + 1) % 8; }
  A a = { tab }; unsigned long h = 0;
  for (int i = 0; i < 8; ++i) { V v = { 1, (unsigned)i }; V r = resolve(a, v); h = h * 31 + r.kind * 100 + r.index; }
  for (long i = -3; i < 4; ++i) { P p = { i, i * 7 }; h = h * 31 + sum(p); P q = { 2 - i, i }; h = h * 31 + first(p, q); }
  for (int k = 0; k < 5; ++k) { T t = { k, 2 * k, 3 * k }; T r = bump(t, k); h = h * 31 + r.x + 10 * r.y + 100 * r.z; }
  for (int i = 0; i < 5; ++i) { V v = { (unsigned char)i, (unsigned)i * 3 }; V r = esc(v); h = h * 31 + r.kind + r.index; }
  printf("%lu\n", h);
}
