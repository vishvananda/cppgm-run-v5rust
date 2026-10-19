// expect exit 0
// Widening a nonthrowing operand segment defers empty cleanup to its try.
int alive, copies, bad;
struct Token {
  Token() { ++alive; }
  ~Token() { --alive; }
};
struct Value {
  Value() {}
  Value(const Value&, Token = Token()) noexcept {
    ++copies;
    if (alive != 1) bad = 1;
  }
  ~Value() {}
};
void probe(int mode) {
  Value first, second;
  try {
    if (mode == 0) throw (0, first);
    if (mode == 1) throw static_cast<Value&&>(first);
    throw static_cast<const Value&>(second);
  } catch (Value&) {
    if (alive) bad = 1;
  }
}
int main() {
  probe(0);
  probe(1);
  probe(2);
  return alive || bad || copies != 3;
}
