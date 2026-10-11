// A multi-level virtual-inheritance diamond: D -> AB -> {A, B} -> virtual V.
// Constructing D drives the recursive construction-vtable group / VTT (AB's
// sub-VTT embeds A's and B's sub-VTTs), and destruction unwinds the same chain.
// Exercises that the shared virtual base V is constructed exactly once and
// reachable through the diamond, and that every subobject is constructed and
// destroyed.

namespace {

int live = 0;

struct V
{
  int tag;
  V() noexcept { tag = 7; ++live; }
  virtual ~V() noexcept { --live; }
};

struct A : virtual V
{
  A() noexcept { ++live; }
  virtual ~A() noexcept { --live; }
};

struct B : virtual V
{
  B() noexcept { ++live; }
  virtual ~B() noexcept { --live; }
};

struct AB : A, B
{
  AB() noexcept { ++live; }
  virtual ~AB() noexcept { --live; }
  virtual void mark() noexcept {}
};

struct D : AB
{
  D() noexcept { ++live; }
  ~D() noexcept { --live; }
};

}  // namespace

int main()
{
  {
    D value;
    if(value.tag != 7) {
      return 1;  // shared virtual base not reachable / not constructed
    }
    if(live != 5) {
      return 2;  // V + A + B + AB + D should all be live
    }
  }
  return live == 0 ? 0 : 3;  // all subobjects destroyed exactly once
}
