// expect exit 0
int steps, fail_at, alive, tokens;
int next() { if (++steps == fail_at) throw steps; return steps; }
struct Token {
  Token() { ++tokens; }
  Token(const Token&) { ++tokens; }
  ~Token() { --tokens; }
};
Token token() { return Token(); }
struct A {
  int value;
  A(int n) : value(n) { ++alive; }
  A(Token) : value(3) { ++alive; }
  A(const A& other) : value(other.value) { ++alive; }
  ~A() { --alive; }
};
A from(int n) { return A(n); }
struct Direct { A a; int n; Direct() : a(A(next())), n(next()) {} };
struct Default { A a = A(next()); int n; Default() : n(next()) {} };
struct Result { A a; int n; Result() : a(from(next())), n(next()) {} };
struct Argument { A a; int n; Argument() : a(A(token())), n(next()) {} };
template<class B> bool check(int fail) {
  steps = 0;
  fail_at = fail;
  try { B b; if (alive != 1 || tokens != 0) return false; }
  catch (int) { if (fail == 0) return false; }
  return alive == 0 && tokens == 0;
}
int main() {
  for (int fail = 0; fail != 3; ++fail) {
    if (!check<Direct>(fail)) return 1;
    if (!check<Default>(fail)) return 2;
    if (!check<Result>(fail)) return 3;
    if (!check<Argument>(fail)) return 4;
  }
  return 0;
}
