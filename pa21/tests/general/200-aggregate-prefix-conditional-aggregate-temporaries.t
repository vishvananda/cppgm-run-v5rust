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
struct E {S a; int n; S b;};
extern "C" void construction_probe(){S x(1),y(2); E e=choice()?E{x,read(S(9)),y}:E{y,read(S(10)),x};step_value();}
extern "C" int maxstep(){return 7;}

int choice() { return limit & 1; }

int main()
{
  for (limit = 0; limit <= maxstep(); ++limit)
  {
    progress = live = 0;
    trace_digest = 0;
    int caught = 0;
    try { construction_probe(); } catch (int n) { caught = n; }
    if (live || caught != (limit == 0 ? 0 : limit)) return 1;

  }
  return 0;
}
