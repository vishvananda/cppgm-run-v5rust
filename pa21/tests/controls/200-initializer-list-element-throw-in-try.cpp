// N3485 15.2p1, 8.5.4p5: when the construction of an element of a
// std::initializer_list's array throws inside a try block or a handler while
// another object is alive, the elements already constructed are destroyed,
// then that object, before the handler runs; expect exit 0
namespace std {
template<class E> class initializer_list {
  const E* b; unsigned long n;
  initializer_list(const E* p, unsigned long k) : b(p), n(k) {}
public:
  initializer_list() : b(0), n(0) {}
  unsigned long size() const { return n; }
};
}
int t, countdown;
struct A {
  int k;
  A(int x) : k(x) { if (--countdown == 0) throw 9; }
  A(const A& o) : k(o.k) { if (--countdown == 0) throw 8; }
  ~A() { t = t * 10 + k; }
};
int sum(std::initializer_list<A> l) { return (int)l.size(); }
void in_handler() {
  try { throw 1; } catch (int) { A o(4); sum({ 1, 2 }); }
}
int main() {
  int bad = 0;
  countdown = 3;
  try { A o(4); sum({ 1, 2 }); } catch (int e) { t = t * 10 + e; }
  bad += t != 149;
  t = 0;
  countdown = 3;
  try { A a(1); sum({ a, a }); } catch (int e) { t = t * 10 + e; }
  bad += (t != 118) * 2;
  t = 0;
  countdown = 4;
  try { A o(5); try { A p(6); sum({ 1, 2, 3 }); } catch (long) { t = 7; } } catch (int e) { t = t * 10 + e; }
  bad += (t != 1659) * 4;
  t = 0;
  countdown = 3;
  try { in_handler(); } catch (int e) { t = t * 10 + e; }
  bad += (t != 149) * 8;
  return bad;
}
