// N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.

// return-value
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
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1), b(2, 1);
    return 0;
  }
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 219;
  }
  return 1;
}
} // namespace case_0

// return-void
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
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1), b(2, 1);
    return;
  }
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 219;
  }
  return 1;
}
} // namespace case_1

// return-shared
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
int f(bool flag) {
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1), b(2, 1);
    if (flag)
      return 1;
    return 0;
  }
}
int check() {
  for (int i = 0; i != 2; ++i) {
    drops = bad = trace = 0;
    try {
      f(i != 0);
      return 1;
    } catch (int n) {
      if (n != 7 || live || bad || drops != 2 || trace != 219)
        return 2;
    }
  }
  return 0;
}
} // namespace case_2

// return-two-active-handlers
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
int f() {
  try {
    throw E();
  } catch (...) {
    try {
      throw E();
    } catch (...) {
      trace = 0;
      G a(1, 2), b(2, 2);
      return 0;
    }
  }
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 2199;
  }
  return 1;
}
} // namespace case_3

// return-unnamed-value-handler
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
int f() {
  try {
    throw E();
  } catch (E) {
    trace = 0;
    G a(1, 2), b(2, 2);
    return 0;
  }
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 2199;
  }
  return 1;
}
} // namespace case_4

// return-enclosing-try-match
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
int check() {
  try {
    try {
      throw E();
    } catch (...) {
      trace = 0;
      G a(1, 1), b(2, 1);
      return 0;
    }
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 219;
  }
  return 1;
}
} // namespace case_5

// return-outer-guard-after-catch
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
void f() {
  G outside(2, 0);
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G inside(1, 1);
    return;
  }
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 192;
  }
  return 1;
}
} // namespace case_6

// matched-try-return-keeps-local-handler
namespace case_7 {
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
  try {
    G a(1, 0), b(2, 0);
    return 9;
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 21;
  }
}
int check() { return f(); }
} // namespace case_7

// nonthrowing-return-control
namespace case_8 {
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
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1), b(2, 1);
    should_throw = 0;
    return 8;
  }
}
int check() { return f() != 8 || live || bad || drops != 2 || trace != 219; }
} // namespace case_8

// nonthrowing-outside-guard-control
namespace case_9 {
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
  G outside(1, 0);
  try {
    throw E();
  } catch (...) {
    trace = 0;
    return;
  }
}
int check() {
  f();
  return live || bad || drops != 1 || trace != 91;
}
} // namespace case_9

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
  if (case_8::check())
    return 9;
  if (case_9::check())
    return 10;
  return 0;
}
