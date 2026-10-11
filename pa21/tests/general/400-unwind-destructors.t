// lowir.md unwind=no: a destructor without an exception-specification is
// non-throwing unless a destructor of a base or member allows exceptions
// (12.4p3, 15.4p14); an implicitly declared destructor likewise.
struct Quiet { int value; ~Quiet() { value = 0; } };
struct Loud { int value; ~Loud() noexcept(false) { value = 0; } };
struct HoldsQuiet { Quiet member; ~HoldsQuiet() {} };
struct HoldsLoud { Loud member; ~HoldsLoud() {} };
struct FromLoud : Loud { int extra; };
struct ImplicitQuiet { Quiet member; };

int main()
{
  HoldsQuiet a;
  HoldsLoud b;
  FromLoud c;
  ImplicitQuiet d;
  a.member.value = 1;
  b.member.value = 2;
  c.value = 3;
  d.member.value = 4;
  return a.member.value + b.member.value + c.value + d.member.value == 10 ? 0 : 1;
}
