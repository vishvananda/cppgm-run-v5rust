// N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.

// function-try-local-return
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
int f() try {
  G a(1, 0), b(2, 0);
  return 8;
} catch (int x) {
  return x;
}
int check() { return f() != 7 || bad || drops != 2 || trace != 21; }
} // namespace case_0

// function-try-constructor-local
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
struct X {
  X() try { G a(1, 0), b(2, 0); } catch (int x) {
    if (x != 7 || trace != 21 || drops != 2)
      bad = 1;
  }
};
int check() {
  try {
    X x;
    return 1;
  } catch (int x) {
    return x != 7 || bad || trace != 21 || drops != 2;
  }
}
} // namespace case_1

// function-try-destructor-local
namespace case_2 {
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
struct X {
  ~X() noexcept(false) try { G a(1, 0), b(2, 0); } catch (int x) {
    if (x != 7 || trace != 21 || drops != 2)
      bad = 1;
  }
};
int check() {
  try {
    X x;
    return 1;
  } catch (int x) {
    return x != 7 || bad || trace != 21 || drops != 2;
  }
}
} // namespace case_2

// constructor-local-before-base
namespace case_3 {
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
struct B {
  B() {}
  ~B() { trace = trace * 10 + 4; }
};
struct X : B {
  X() { G a(1, 0), b(2, 0); }
};
int check() {
  try {
    X x;
    return 1;
  } catch (int v) {
    return v != 7 || bad || drops != 2 || trace != 214;
  }
}
} // namespace case_3

// function-try-constructor-base
namespace case_4 {
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
struct B {
  B() {}
  ~B() { trace = trace * 10 + 4; }
};
struct X : B {
  X() try { G a(1, 0), b(2, 0); } catch (int v) {
    if (v != 7 || trace != 214 || drops != 2)
      bad = 1;
  }
};
int check() {
  try {
    X x;
    return 1;
  } catch (int v) {
    return v != 7 || bad || drops != 2 || trace != 214;
  }
}
} // namespace case_4

// destructor-local-before-base
namespace case_5 {
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
struct B {
  B() {}
  ~B() { trace = trace * 10 + 4; }
};
struct X : B {
  ~X() noexcept(false) { G a(1, 0), b(2, 0); }
};
int check() {
  try {
    X x;
    return 1;
  } catch (int v) {
    return v != 7 || bad || drops != 2 || trace != 214;
  }
}
} // namespace case_5

// function-try-destructor-base
namespace case_6 {
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
struct B {
  B() {}
  ~B() { trace = trace * 10 + 4; }
};
struct X : B {
  ~X() noexcept(false) try { G a(1, 0), b(2, 0); } catch (int v) {
    if (v != 7 || trace != 214 || drops != 2)
      bad = 1;
  }
};
int check() {
  try {
    X x;
    return 1;
  } catch (int v) {
    return v != 7 || bad || drops != 2 || trace != 214;
  }
}
} // namespace case_6

// member-function-try-local
namespace case_7 {
int drops, trace;
struct G {
  int id;
  G(int n) : id(n) {}
  ~G() noexcept(false) {
    ++drops;
    trace = trace * 10 + id;
    if (id == 2)
      throw 7;
  }
};
struct X {
  int f(int) try {
    G a(1), b(2);
    return 8;
  } catch (int v) {
    return v;
  }
};
int check() {
  X x;
  return x.f(0) != 7 || drops != 2 || trace != 21;
}
} // namespace case_7

int main() {
  if (case_0::check())
    return 1;
  if (case_1::check())
    return 2;
  if (case_2::check())
    return 3;
  if (case_3::check())
    return 4;
  if (case_4::check())
    return 5;
  if (case_5::check())
    return 6;
  if (case_6::check())
    return 7;
  if (case_7::check())
    return 8;
  return 0;
}
