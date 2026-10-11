// lowir.md unwind=no: a template's noexcept(constant-expression) is evaluated
// for each specialization (15.4p1, 14.8.2p7): f<int> is non-throwing where int
// has four bytes, f<char> is not.
template<class X> int width(X x) noexcept(sizeof(X) == 4) { return sizeof(x); }
template<class X> struct Holder
{
  X value;
  int get() const noexcept(sizeof(X) == 8) { return static_cast<int>(value); }
};

int main()
{
  Holder<long> wide;
  Holder<int> narrow;
  wide.value = 2;
  narrow.value = 3;
  return width(1) + width('c') + wide.get() + narrow.get() == 10 ? 0 : 1;
}
