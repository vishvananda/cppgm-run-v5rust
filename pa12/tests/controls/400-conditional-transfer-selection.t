// Both selected arms retain their value category during result construction.
int copies, moves;
struct Value {
  int n;
  Value(int x) : n(x) {}
  Value(const Value& v) : n(v.n) { ++copies; }
  Value(Value&& v) : n(v.n) { ++moves; }
};
struct Result { Value member; Result() : member(7) {} };
Value choose(bool local, bool nested) {
  Value base(3);
  return nested ? (local ? base : Result().member) :
    (local ? base : Result().member);
}
int check(bool local, bool nested) {
  copies = moves = 0;
  Value result = choose(local, nested);
  return result.n != (local ? 3 : 7) || copies != (local ? 1 : 0) ||
    (!local && moves == 0);
}
int main() {
  return check(false, false) || check(false, true) ||
    check(true, false) || check(true, true);
}
