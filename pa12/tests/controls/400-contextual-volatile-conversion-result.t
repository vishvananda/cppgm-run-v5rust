// N3485 4p3, 4.1, 12.3.2: contextual truth reads a conversion's
// volatile reference result as a qualified glvalue, exactly once per use.
// expect exit 0
volatile int value;
int calls;
struct Box {
  ~Box() {}
  operator volatile int&() { ++calls; return value; }
};
int negate(Box& x) { return !x; }
int both(Box& x) { return x && true; }
int either(Box& x) { return x || false; }
int select(Box& x) { return x ? 2 : 3; }
int main() {
  Box x;
  value = 1;
  if (negate(x) != 0 || both(x) != 1 || either(x) != 1) return 1;
  if (select(x) != 2 || calls != 4) return 2;
  value = 0;
  if (negate(x) != 1 || both(x) != 0 || either(x) != 0) return 3;
  if (select(x) != 3 || calls != 8) return 4;
  if (x) return 5;
  if (calls != 9) return 6;
  return 0;
}
