// N3485 12.8p32: named automatic objects use two-pass selection; expect exit 0
int moves;
int copies;
struct Value
{
  int n;
  Value(int v = 0) : n(v) {}
  Value(const Value& v) : n(v.n) { ++copies; }
  Value(Value&& v) : n(v.n) { ++moves; v.n = 0; }
};
Value parameter(Value v) { return ((v)); }
Value reference(Value&& v) { return v; }
Value local(bool first)
{
  if (first) { Value a(17); return a; }
  Value b(23);
  return b;
}
struct Fallback
{
  int n;
  Fallback(int v = 0) : n(v) {}
  Fallback(Fallback& v) : n(v.n) { ++copies; }
  Fallback(const Fallback&&) { n = 91; }
  Fallback(volatile Fallback&&) { n = 92; }
};
Fallback fallback(Fallback v) { return v; }
int main()
{
  Value a(7);
  Value b = parameter(a);
  Value c = reference(static_cast<Value&&>(a));
  Value d = local(true);
  Value e = local(false);
  Fallback f(31);
  int value = fallback(f).n;
  return a.n != 7 || b.n != 7 || c.n != 7 || d.n != 17 || e.n != 23
    || value != 31 || moves != 3 || copies != 4;
}
