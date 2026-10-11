// N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.

// ordinary-return-tail
namespace case_0 {
int live, drops, bad, trace, should_throw = 1;
struct E {
  E() { ++live; }
  E(E const &) { ++live; }
  ~E() {
    --live;
    trace = trace * 10 + 9;
  }
};
struct G {
  int id, expected;
  G(int n, int e) : id(n), expected(e) {}
  ~G() noexcept(false) {
    if (live != expected)
      bad = 1;
    ++drops;
    trace = trace * 10 + id;
    if (id == 2 && should_throw)
      throw 7;
  }
};
int f() {
  G a(1, 0), b(2, 0);
  return 8;
}
int check() {
  try {
    f();
    return 1;
  } catch (int x) {
    return x != 7 || bad || trace != 21 || drops != 2;
  }
}
} // namespace case_0

// ordinary-scope-tail
namespace case_1 {
int live, drops, bad, trace, should_throw = 1;
struct E {
  E() { ++live; }
  E(E const &) { ++live; }
  ~E() {
    --live;
    trace = trace * 10 + 9;
  }
};
struct G {
  int id, expected;
  G(int n, int e) : id(n), expected(e) {}
  ~G() noexcept(false) {
    if (live != expected)
      bad = 1;
    ++drops;
    trace = trace * 10 + id;
    if (id == 2 && should_throw)
      throw 7;
  }
};
void f() {
  {
    G a(1, 0), b(2, 0);
  }
}
int check() {
  try {
    f();
    return 1;
  } catch (int x) {
    return x != 7 || bad || trace != 21 || drops != 2;
  }
}
} // namespace case_1

// Abandoned NRVO and prvalue/call/conditional results (CWG 2176).
namespace case_2 {
int live, drops, trace, bad;
struct R {
  R() { ++live; }
  R(R const &) { ++live; }
  ~R() {
    --live;
    ++drops;
    trace = trace * 10 + 1;
  }
};
struct G {
  ~G() noexcept(false) {
    trace = trace * 10 + 2;
    throw 7;
  }
};
R f() {
  R r;
  G g;
  return r;
}
struct Earlier {
  ~Earlier() { if (live) bad = 1; }
};
R make() { return R(); }
R value(int mode) {
  Earlier earlier;
  G g;
  if (mode == 0) return R();
  if (mode == 1) return make();
  return mode == 2 ? R() : R();
}
int check() {
  try {
    R r = f();
    return 1;
  } catch (int v) {
    if (v != 7 || live || drops != 1 || trace != 21) return 1;
  }
  for (int mode = 0; mode != 3; ++mode) {
    drops = trace = bad = 0;
    try {
      R r = value(mode);
      return 1;
    } catch (int v) {
      if (v != 7 || live || drops < 1 || bad) return 1;
    }
  }
  return 0;
}
} // namespace case_2

int main() {
  if (case_0::check())
    return 1;
  if (case_1::check())
    return 2;
  if (case_2::check())
    return 3;
  return 0;
}
