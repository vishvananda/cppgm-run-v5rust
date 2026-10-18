// expect exit 0
// Nested scalar choices keep the outer branch's temporary lifetime policy.
int trace;
struct X {
  X() { trace = trace * 10 + 1; }
  ~X() noexcept { trace = trace * 10 + 4; }
  operator int() const noexcept { return 1; }
};
struct Y {
  Y() { trace = trace * 10 + 2; }
  ~Y() noexcept { trace = trace * 10 + 5; }
  operator int() const noexcept { return 2; }
};
struct Throwing {
  Throwing() { trace = trace * 10 + 1; }
  ~Throwing() noexcept(false) { trace = trace * 10 + 4; }
  operator int() const { return 1; }
};
int nested_then(bool a, bool b) {
  return a ? (b ? int{X()} : int{Y()}) : int{X()};
}
int nested_else(bool a, bool b) {
  return a ? int{X()} : (b ? int{X()} : int{Y()});
}
int siblings(bool a, bool b) {
  return a ? (b ? int{X()} : int{Y()}) : (b ? int{Y()} : int{X()});
}
int condition_temporary(bool a) {
  return a ? (int{X()} ? int{Y()} : 3) : 4;
}
int throwing(bool a, bool b) {
  return a ? (b ? int{Throwing()} : int{Y()}) : int{X()};
}
int user_converted(bool a, bool b) {
  return a ? (b ? X() : X()) : 7;
}
int tracked_conversion(bool a, bool b) {
  return (a ? (b ? X() : X()) : 7) + int{Y()};
}
int main() {
  trace = 0;
  if (nested_then(true, true) != 1 || trace != 14) return 1;
  trace = 0;
  if (nested_then(true, false) != 2 || trace != 25) return 2;
  trace = 0;
  if (nested_then(false, false) != 1 || trace != 14) return 3;
  trace = 0;
  if (nested_else(false, false) != 2 || trace != 25) return 4;
  trace = 0;
  if (siblings(false, true) != 2 || trace != 25) return 5;
  trace = 0;
  if (siblings(true, false) != 2 || trace != 25) return 6;
  trace = 0;
  if (condition_temporary(true) != 2 || trace != 1254) return 7;
  trace = 0;
  if (throwing(true, true) != 1 || trace != 14) return 8;
  trace = 0;
  if (user_converted(true, false) != 1 || trace != 14) return 9;
  trace = 0;
  if (tracked_conversion(true, false) != 3 || trace != 1254) return 10;
  return 0;
}
