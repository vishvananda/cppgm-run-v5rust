// lowir.md alias=noalias: a same-class copy or move constructor's `this` and
// source state alias=noalias (they are distinct live objects during the
// construction), one with further defaulted parameters included (12.8p2); a
// converting constructor and an assignment operator do not.
struct Value
{
  long data;
  Value(long x) : data(x) {}
  Value(const Value& other, int scale = 1) : data(other.data * scale) {}
  Value(Value&& other) : data(other.data) { other.data = 0; }
  Value& operator=(const Value& other) { data = other.data; return *this; }
};

int main()
{
  Value a(4);
  Value b(a);
  Value c(static_cast<Value&&>(b));
  Value d(c, 2);
  a = d;
  return a.data == 8 && b.data == 0 && c.data == 4 ? 0 : 1;
}
