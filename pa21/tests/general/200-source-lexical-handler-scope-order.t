// N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.

// normal-handler-end
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
void f() {
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1), b(2, 1);
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

// nested-block-end
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
    G a(1, 1);
    {
      G b(2, 1);
    }
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

// break-exits-handler
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
void f() {
  do {
    try {
      throw E();
    } catch (...) {
      trace = 0;
      G a(1, 1), b(2, 1);
      break;
    }
  } while (false);
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 219;
  }
  return 1;
}
} // namespace case_2

// continue-exits-handler
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
void f() {
  for (int i = 0; i != 1; ++i) {
    try {
      throw E();
    } catch (...) {
      trace = 0;
      G a(1, 1), b(2, 1);
      continue;
    }
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
} // namespace case_3

// goto-exits-handler
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
void f() {
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1), b(2, 1);
    goto finish;
  }
finish:;
}
int check() {
  try {
    f();
  } catch (int n) {
    return n != 7 || live || bad || drops != 2 || trace != 219;
  }
  return 1;
}
} // namespace case_4

// break-within-handler
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
void f() {
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1);
    do {
      G b(2, 1);
      break;
    } while (false);
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
} // namespace case_5

// continue-within-handler
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
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1);
    for (int i = 0; i != 1; ++i) {
      G b(2, 1);
      continue;
    }
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
} // namespace case_6

// goto-within-handler
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
void f() {
  try {
    throw E();
  } catch (...) {
    trace = 0;
    G a(1, 1);
    {
      G b(2, 1);
      goto finish;
    }
  finish:;
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
