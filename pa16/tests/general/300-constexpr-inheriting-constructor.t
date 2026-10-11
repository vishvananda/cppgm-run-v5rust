// An inheriting constructor (12.9) is constexpr when the user-written
// constructor its implicit definition performs (12.9p8) would satisfy
// 7.1.5p4: the inherited constructor is constexpr, and every other
// subobject is initialized by a constexpr constructor or, for a member
// with a brace-or-equal-initializer, by a constant expression. Its class
// is then a literal type (3.9p10): a constexpr object, a static_assert, an
// array bound and a template argument construct through it, with the
// inherited constructor's default arguments, in a class template and
// through a chain of inheriting constructors, and an object of static
// storage duration it initializes is constant-initialized (3.6.2p2) before
// any dynamic initialization. A member initializer that is no constant
// expression makes it no constexpr constructor, not an error: its object
// is initialized dynamically, in order. Its exception-specification stays
// the one 15.4p14 gives it.
int calls;
int counted() { return ++calls; }

struct Base
{
  int v;
  constexpr Base(int x) : v(x) {}
};

struct Plain : Base { using Base::Base; };

struct Inner
{
  int k;
  constexpr Inner() : k(7) {}
};

struct Members : Base
{
  using Base::Base;
  int m = 10;
  int n{v + m};
  Inner inner;
};

struct Defaults
{
  int sum;
  constexpr Defaults(int a, int b = 20) : sum(a + b) {}
};
struct Defaulted : Defaults { using Defaults::Defaults; };

template <class B> struct Wrapped : B { using B::B; int extra = 3; };

struct Chained : Members { using Members::Members; int z = 4; };

struct Counted : Base { using Base::Base; int m = counted(); };

template <int N> struct Size { static const int value = N; };

constexpr Plain plain(1);
static_assert(plain.v == 1, "a constexpr object");
static_assert(Plain(2).v == 2, "a temporary in a static_assert");
int bound[Plain(3).v];
static_assert(Size<Plain(4).v>::value == 4, "a template argument");
constexpr Members members(1);
static_assert(members.m == 10 && members.n == 11 && members.inner.k == 7, "member initializers");
constexpr Defaulted shorter(1);
constexpr Defaulted longer(1, 2);
static_assert(shorter.sum == 21 && longer.sum == 3, "default arguments");
constexpr Wrapped<Base> wrapped(5);
static_assert(wrapped.v == 5 && wrapped.extra == 3, "a class template");
constexpr Chained chained(6);
static_assert(chained.v == 6 && chained.n == 16 && chained.z == 4, "a chain");
static_assert(noexcept(Plain(1)) == noexcept(Base(1)), "the base constructor's exception-specification");
static_assert(!noexcept(Counted(1)), "a member initializer that may throw");

int read_static();
int read_dynamic();
// Dynamic initializations, in order: the first two run before the objects
// they read are defined.
int early_static = read_static();
int early_dynamic = read_dynamic();
Wrapped<Base> constant_initialized(30);
Defaulted defaulted_static(1);
Counted dynamic(40);
int late_dynamic = read_dynamic();
int read_static() { return constant_initialized.v + constant_initialized.extra + defaulted_static.sum; }
int read_dynamic() { return dynamic.m; }

int main()
{
  if (plain.v != 1 || sizeof bound != 3 * sizeof(int) || members.n != 11) return 1;
  if (shorter.sum != 21 || wrapped.extra != 3 || chained.z != 4) return 2;
  if (early_static != 54) return 3;
  if (early_dynamic != 0 || late_dynamic != 1 || dynamic.v != 40 || calls != 1) return 4;
  constexpr Chained local(8);
  static_assert(local.n == 18, "a constexpr automatic object");
  Counted runtime(9);
  return runtime.m == 2 && runtime.v == 9 ? 0 : 5;
}
