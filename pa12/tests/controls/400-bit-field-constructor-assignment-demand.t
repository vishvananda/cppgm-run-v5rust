// expect exit 0
// PA12: implicit assignment demand selects constructor bit-field ordering.
int calls;
int next() { return ++calls; }

struct Bits {
  unsigned a : 2;
  unsigned b : 3;
  Bits() : a(next()), b(next()) {}
};
struct Tagged : Bits { char tag; };

struct Signed {
  signed char a : 3;
  signed char b : 4;
  Signed(int x) : a(x), b(x - 1) {}
};

struct Volatile {
  volatile unsigned a : 2;
  volatile unsigned b : 3;
  Volatile(int x) : a(x), b(x + 1) {}
};

struct Packed {
  unsigned a : 3;
  unsigned long b : 62;
  Packed() : a(3), b(17) {}
} __attribute__((packed));

struct Anonymous {
  char tag;
  struct { unsigned a : 2; unsigned b : 3; };
  Anonymous(int x) : tag(x + 2), a(x), b(x + 1) {}
};

struct Separated {
  unsigned a : 2;
  unsigned : 0;
  unsigned b : 3;
  Separated(int x) : a(x), b(x + 1) {}
};

int main() {
  Tagged a, b;
  a.tag = 7;
  b.tag = 9;
  if (calls != 4 || a.a != 1 || a.b != 2 || b.a != 3 || b.b != 4)
    return 1;
  static_cast<Bits&>(b) = a;
  if (b.a != 1 || b.b != 2 || b.tag != 9) return 2;
  static_cast<Bits&>(a) = static_cast<Bits&&>(b);
  if (a.a != 1 || a.b != 2 || a.tag != 7) return 3;
  Signed s(-2), t(1);
  t = s;
  if (t.a != -2 || t.b != -3) return 4;
  Volatile v(1), w(2);
  w = static_cast<Volatile&&>(v);
  if (w.a != 1 || w.b != 2) return 5;
  Packed p, q;
  q = p;
  if (q.a != 3 || q.b != 17) return 6;
  Anonymous x(1), y(2);
  y = x;
  if (y.tag != 3 || y.a != 1 || y.b != 2) return 7;
  Separated i(1), j(2);
  j = i;
  if (j.a != 1 || j.b != 2) return 8;
  return 0;
}
