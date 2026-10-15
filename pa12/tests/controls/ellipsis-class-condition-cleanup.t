// expect exit 0
int take(...) { return 2; }
struct Value {
  int n;
  Value() : n(3) {}
  ~Value() {}
};
int main() { if (take(Value())) return 0; return 1; }
