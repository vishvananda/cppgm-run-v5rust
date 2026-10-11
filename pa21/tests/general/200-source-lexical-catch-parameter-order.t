// N3485 [except.handle] and [except.ctor]: lexical lifetime and unwind order.

// named-catch-fallthrough
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
int check() {
  try {
    throw E();
  } catch (E e) {
    trace = 0;
    G a(1, 2);
  }
  return live || bad || drops != 1 || trace != 199;
}
} // namespace case_0

// catch-parameter-throws-named
namespace case_1 {
int live, drops, trace;
struct E {
  int copy;
  E() : copy(0) { ++live; }
  E(E const &) : copy(1) { ++live; }
  ~E() noexcept(false) {
    --live;
    ++drops;
    trace = trace * 10 + 9;
    if (copy)
      throw 5;
  }
};
int check() {
  try {
    try {
      throw E();
    } catch (E e) {
      trace = 1;
    }
  } catch (int v) {
    return v != 5 || live || drops != 2 || trace != 199;
  }
  return 1;
}
} // namespace case_1

// catch-parameter-throws-unnamed
namespace case_2 {
int live, drops, trace;
struct E {
  int copy;
  E() : copy(0) { ++live; }
  E(E const &) : copy(1) { ++live; }
  ~E() noexcept(false) {
    --live;
    ++drops;
    trace = trace * 10 + 9;
    if (copy)
      throw 5;
  }
};
int check() {
  try {
    try {
      throw E();
    } catch (E) {
      trace = 1;
    }
  } catch (int v) {
    return v != 5 || live || drops != 2 || trace != 199;
  }
  return 1;
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
