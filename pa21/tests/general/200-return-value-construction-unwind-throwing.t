// N3485 15.2p1, 6.6.3p2: the constructor or call that builds a class return
// value in the caller's destination may throw; the function's automatic
// objects are then destroyed, last first, before the exception leaves it.
// A conditional's arms each unwind the same objects, and a destructor that
// may throw during the return's cleanup still destroys the built result.
int live, order, throw_at, made;
struct R {
  int v;
  R() : v(0) { step(); ++live; }
  R(int x) : v(x) { step(); ++live; }
  R(const R& o) : v(o.v) { step(); ++live; }
  ~R() { --live; }
  static void step() { if (++made == throw_at) throw 1; }
};
struct H { H() { ++live; } ~H() { --live; order = order * 10 + 1; } };
struct G { G() { ++live; } ~G() noexcept(false) { --live; order = order * 10 + 2; } };
struct Box { R member; Box(int x) : member(x) { ++live; } ~Box() { --live; } };
R make() { return R(5); }
R constructed(int m) { H h; if (m == 0) return R(); return R(); }
R copied(int m) { H h; G g; R r; R s; if (m) return r; return s; }
R converted(int m) { H h; G g; return R(m); }
R called(int m) { H h; G g; return m ? make() : make(); }
R parameter(int m, R p) { G g; return p; }
R member(int m) { R base(1); G g; return m ? base : Box(7).member; }
void run_constructed() { constructed(0); }
void run_copied() { copied(1); }
void run_converted() { converted(1); }
void run_called() { called(1); }
void run_parameter() { R p(1); parameter(1, p); }
void run_member() { member(0); }
int check(void (*f)(), int at, int expect_order)
{
  live = order = made = 0;
  throw_at = at;
  try { f(); } catch (int) { return live == 0 && order == expect_order ? 0 : 1; }
  return 1;
}
int main()
{
  return check(run_constructed, 1, 1) + 2 * check(run_copied, 3, 21) +
    4 * check(run_converted, 1, 21) + 8 * check(run_called, 1, 21) +
    16 * check(run_parameter, 3, 2) + 32 * check(run_member, 3, 2);
}
