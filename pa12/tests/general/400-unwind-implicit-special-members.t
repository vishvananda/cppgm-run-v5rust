// lowir.md unwind=no: an implicitly declared special member function, or one
// defaulted on its first declaration, is non-throwing when every function it
// directly invokes is (15.4p14).
struct Safe
{
  long value;
  Safe() noexcept : value(1) {}
  Safe(const Safe& other) noexcept : value(other.value) {}
  ~Safe() {}
};
struct Risky
{
  long value;
  Risky() : value(2) {}
  Risky(const Risky& other) : value(other.value) {}
  ~Risky() {}
};
struct HoldsSafe { Safe member; };
struct HoldsRisky { Risky member; };
struct DefaultedSafe
{
  Safe member;
  DefaultedSafe() = default;
  DefaultedSafe(const DefaultedSafe&) = default;
};

int main()
{
  HoldsSafe a;
  HoldsSafe b(a);
  HoldsRisky c;
  HoldsRisky d(c);
  DefaultedSafe e;
  DefaultedSafe f(e);
  return b.member.value + d.member.value + f.member.value == 4 ? 0 : 1;
}
