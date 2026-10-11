struct S { int id; S(); S(int); S(const S&); ~S() noexcept; };
static int progress, limit, live;
static unsigned trace_digest;
static int next()
{
  if (++progress == limit) throw progress;
  trace_digest = trace_digest * 33u + progress;
  ++live;
  return progress;
}
S::S() : id(next()) {}
S::S(int) : id(next()) {}
S::S(const S&) : id(next()) {}
S::~S() noexcept
{
  trace_digest = trace_digest * 33u - id;
  --live;
}
int step_value() { if (++progress == limit) throw progress; return 10; }
int read(const S&) { return step_value(); }
extern int step_value(); extern int read(const S&); extern int choice();
struct E {S a; int n; S b; int m;};
extern "C" void construction_probe(){S x(1),y(2); E e={x,choice()&&read(S(9)),y,step_value()};}
extern "C" int maxstep(){return 7;}

int choice() { return 1; }

static const unsigned expected[] = {
  4083490624u, 0u, 32u, 38048u, 41611712u, 2373708736u, 2373708736u, 4081190656u
};
int main()
{
  for (limit = 0; limit <= maxstep(); ++limit)
  {
    progress = live = 0;
    trace_digest = 0;
    int caught = 0;
    try { construction_probe(); } catch (int n) { caught = n; }
    if (live || caught != (limit == 0 ? 0 : limit)) return 1;
    if (trace_digest != expected[limit]) return 2;
  }
  return 0;
}
