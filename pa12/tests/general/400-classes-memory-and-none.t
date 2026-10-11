// lowir.md classes: `none` names an eightbyte that holds no member (an empty
// class, padding); `memory`, on a parameter only, says the object is passed in
// memory: an object with an x87 (long double) member. Such a result states no
// classes (the psABI returns it in st0, which these classes do not describe).
struct Empty {};
struct DoubleEmpty { double d; Empty e; };
struct FloatEmpty { float f; Empty e; };
struct Extended { long double x; };

int take_empty(Empty) { return 1; }
Empty make_empty() { return Empty(); }
double take_double_empty(DoubleEmpty p) { return p.d; }
DoubleEmpty make_double_empty(double x) { DoubleEmpty r; r.d = x; return r; }
float take_float_empty(FloatEmpty p) { return p.f; }
long double take_extended(Extended p) { return p.x; }
Extended make_extended(long double x) { Extended r = {x}; return r; }

int main()
{
  Empty e = make_empty();
  DoubleEmpty de = make_double_empty(2.0);
  FloatEmpty fe;
  fe.f = 3.0f;
  Extended x = make_extended(4.0L);
  return take_empty(e) == 1 && take_double_empty(de) == 2.0 && take_float_empty(fe) == 3.0f &&
    take_extended(x) == 4.0L ? 0 : 1;
}
