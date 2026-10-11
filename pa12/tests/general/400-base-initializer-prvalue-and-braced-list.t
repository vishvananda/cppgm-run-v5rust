// A base initialized from a prvalue of its own class (a copy the compiler
// elides) takes that value, and a braced list aggregate-initializes an
// aggregate base, zeroing the members it does not name.
inline void* operator new(unsigned long, void* place) noexcept { return place; }
struct Value { int a; Value(int x) : a(x) {} };
struct FromValue : Value { FromValue() : Value(Value(5)) {} };
struct Plain { int a; int b; };
Plain make_plain()
{
  Plain plain;
  plain.a = 6;
  plain.b = 7;
  return plain;
}
struct FromCall : Plain { FromCall() : Plain(make_plain()) {} };
struct Braced : Plain { Braced() : Plain{3, 4} {} };
struct Partial : Plain { Partial() : Plain{5} {} };

alignas(16) unsigned char buffer[32];

int main()
{
  FromValue value;
  if (value.a != 5) return 1;
  FromCall call;
  if (call.a != 6 || call.b != 7) return 2;
  Braced braced;
  if (braced.a != 3 || braced.b != 4) return 3;
  for (unsigned i = 0; i < sizeof buffer; ++i) buffer[i] = 0x55;
  Partial* partial = new (buffer) Partial;
  if (partial->a != 5 || partial->b != 0) return 4;
  return 0;
}
