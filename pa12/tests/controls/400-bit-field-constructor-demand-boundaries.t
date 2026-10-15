// expect exit 0
// Assignment use is independent of output demand and applies to its own class.
struct Used {
  unsigned a : 2;
  unsigned b : 3;
  Used() : a(1), b(2) {}
};
inline void uncalled(Used& a, const Used& b) { a = b; }

struct Unevaluated {
  unsigned a : 2;
  unsigned b : 3;
  Unevaluated() : a(2), b(3) {}
};
void unevaluated(Unevaluated& a, const Unevaluated& b) {
  (void)sizeof(a = b);
}

struct Defaulted {
  unsigned a : 2;
  unsigned b : 3;
  Defaulted() : a(3), b(4) {}
  Defaulted& operator=(const Defaulted&) = default;
};

struct Member { unsigned a : 2; unsigned b : 3; };
struct Aggregate {
  Member m;
  unsigned a : 2;
  unsigned b : 3;
  Aggregate() : m{1, 2}, a(2), b(3) {
    Member local{3, 4};
    a = local.a;
  }
};

struct Unused {
  unsigned a : 2;
  unsigned b : 3;
  Unused() : a(1), b(5) {}
};

int main() {
  Used a;
  Unevaluated b;
  Defaulted c, d;
  c = d;
  Aggregate e, f;
  e = f;
  Unused g;
  if (a.a != 1 || a.b != 2) return 1;
  if (b.a != 2 || b.b != 3) return 2;
  if (c.a != 3 || c.b != 4) return 3;
  if (e.m.a != 1 || e.m.b != 2 || e.a != 3 || e.b != 3) return 4;
  if (g.a != 1 || g.b != 5) return 5;
  return 0;
}
