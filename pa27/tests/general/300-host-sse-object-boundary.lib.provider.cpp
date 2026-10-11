struct SD { double d; long l; };
struct FF { float a, b, c; };
struct DD { double x, y; };
struct LD { long l; double d; };
struct IF { int i; float f; };
struct AL { alignas(16) double d; };
struct LDB { long double x; };
struct E {};

SD make(double d, long l);

SD host_make(double d, long l) { SD s = make(d, l); return s; }
double host_use(SD s, FF f, DD d, LD x, AL a, LDB b)
{
  return s.d + s.l + f.a + f.b + f.c + d.x + d.y + x.l + x.d + a.d + (double)b.x;
}
DD host_swap(DD v) { DD r = {v.y, v.x}; return r; }
double (*host_pointer)(SD, FF, DD, LD, AL, LDB) = host_use;
double host_many(DD a, DD b, DD c, DD d, DD e, double f)
{
  return a.x + b.y + c.x + d.y + e.x + f;
}
double host_calls_back(double (*callback)(SD, FF, IF, E, int))
{
  SD s = {1.5, 2};
  FF f = {0.5f, 9, 0.25f};
  IF i = {3, 0.75f};
  return callback(s, f, i, E(), 100);
}
LD host_mix_back(LD (*callback)(LD, IF))
{
  LD a = {10, 0.5};
  IF b = {3, 0.25f};
  return callback(a, b);
}
