struct V { int value; };
struct B : virtual V { virtual void anchor() {} };
struct D : B {};
int read(B & b) { return b.value; }
int main() {
  D d;
  d.value = 7;
  B * p = &d;
  return read(*p) != 7;
}
