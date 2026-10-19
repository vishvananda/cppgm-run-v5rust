// expect exit 0
// A throwing class-result arm initializes no destination; both-void arms
// retain the frozen dead joins. The selected value arm owns one live result.
int alive;
struct Value {
  int value;
  Value(int n) : value(n) { ++alive; }
  Value(const Value& other) : value(other.value) { ++alive; }
  ~Value() { --alive; }
};
Value choose_left(bool raise) { return raise ? throw 4 : Value(5); }
Value choose_right(bool take) { return take ? Value(6) : throw 7; }
void both_throw(bool choose) { choose ? throw 1 : throw 2; }
int main() {
  { Value left = choose_left(false); Value right = choose_right(true);
    if (left.value != 5 || right.value != 6 || alive != 2) return 1;
  }
  return alive;
}
