// C++11 completed aggregate prefixes: one harness, original trace digests.
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
int choice() { return limit & 1; }

// 200-aggregate-prefix-constructor-member-array
namespace case_0 {
struct H { S first; S e[3]; S last; H(const S&x):first(x),e{x,x,x},last(x){} };
void probe(){S x(1); H h(x);}
int maxstep(){return 6;}
static const unsigned expected[] = {
  1988700000u, 0u, 32u, 38048u, 41611712u, 2373708736u, 4043241184u
};
}

// 200-aggregate-prefix-flat-members
namespace case_1 {
struct E { S a,b,c; };
void probe(){S x(1),y(2),z(3); E e={x,y,z};}
int maxstep(){return 6;}
static const unsigned expected[] = {
  1988700000u, 0u, 32u, 38048u, 41611712u, 2373708736u, 4043241184u
};
}

// 200-aggregate-prefix-long-conditional-prefix
namespace case_2 {
struct E { S a; int n[12]; S b; int last; };
void probe(){S x(1),y(2); E e={x,{choice()?read(S(0)):read(S(100)),choice()?read(S(1)):read(S(101)),choice()?read(S(2)):read(S(102)),choice()?read(S(3)):read(S(103)),choice()?read(S(4)):read(S(104)),choice()?read(S(5)):read(S(105)),choice()?read(S(6)):read(S(106)),choice()?read(S(7)):read(S(107)),choice()?read(S(8)):read(S(108)),choice()?read(S(9)):read(S(109)),choice()?read(S(10)):read(S(110)),choice()?read(S(11)):read(S(111))},y,step_value()};}
int maxstep(){return 29;}
static const unsigned expected[] = {
  1378408384u, 0u, 32u, 38048u, 41611712u, 2373708736u, 2373708736u, 4081190656u, 4081190656u, 4160949248u, 4160949248u, 1679950656u, 1679950656u, 149239104u, 149239104u, 2141989504u, 2141989504u, 3953503104u, 3953503104u, 101040320u, 101040320u, 3166417600u, 3166417600u, 2879916544u, 2879916544u, 3092418304u, 3092418304u, 404769344u, 404769344u, 151618624u
};
}

// 200-aggregate-prefix-matrix-member
namespace case_3 {
struct E { S a[2][2]; };
void probe(){S x(1); E e={{{x,x},{x,x}}};}
int maxstep(){return 5;}
static const unsigned expected[] = {
  4043241184u, 0u, 32u, 38048u, 41611712u, 2373708736u
};
}

// 200-aggregate-prefix-nested-member-array
namespace case_4 {
struct I { S a[2]; }; struct E { I i; S c; };
void probe(){S x(1),y(2),z(3); E e={{{x,y}},z};}
int maxstep(){return 6;}
static const unsigned expected[] = {
  1988700000u, 0u, 32u, 38048u, 41611712u, 2373708736u, 4043241184u
};
}

// 200-aggregate-prefix-template-aggregate-owner
namespace case_5 {
template<class T> struct E {T a; int n; T b;}; template<class T> void use(const T&x,const T&y){E<T> e={x,read(S(9)),y};step_value();}
void probe(){S x(1),y(2);use(x,y);}
int maxstep(){return 7;}
static const unsigned expected[] = {
  4083490624u, 0u, 32u, 38048u, 41611712u, 2373708736u, 2373708736u, 4083490624u
};
}

// 200-aggregate-prefix-temporary-and-final-scalar
namespace case_6 {
struct E { S a; int n; S b; int m; };
void probe(){S x(1),y(2); E e={x,read(S(9)),y,step_value()};}
int maxstep(){return 7;}
static const unsigned expected[] = {
  4083490624u, 0u, 32u, 38048u, 41611712u, 2373708736u, 2373708736u, 4081190656u
};
}

int run(void (*probe)(), int steps, const unsigned* expected) {
  for (limit = 0; limit <= steps; ++limit) {
    progress = live = 0;
    trace_digest = 0;
    int caught = 0;
    try { probe(); } catch (int n) { caught = n; }
    if (live || caught != (limit == 0 ? 0 : limit)) return 1;
    if (trace_digest != expected[limit]) return 2;
  }
  return 0;
}
int main() {
  if (run(case_0::probe, case_0::maxstep(), case_0::expected)) return 1;
  if (run(case_1::probe, case_1::maxstep(), case_1::expected)) return 2;
  if (run(case_2::probe, case_2::maxstep(), case_2::expected)) return 3;
  if (run(case_3::probe, case_3::maxstep(), case_3::expected)) return 4;
  if (run(case_4::probe, case_4::maxstep(), case_4::expected)) return 5;
  if (run(case_5::probe, case_5::maxstep(), case_5::expected)) return 6;
  if (run(case_6::probe, case_6::maxstep(), case_6::expected)) return 7;
  return 0;
}
