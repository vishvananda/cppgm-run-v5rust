// expect exit 0
// A subscript's emitted call protects live objects even when the segment
// scan only visits its operands. A throwing operand keeps its own segment.
int destroyed;
int calls;
struct Guard { ~Guard() { ++destroyed; } };
struct Table {
  int value;
  int& operator[](int) { ++calls; return value; }
};
int index() { return 0; }
int plain(Table& table) {
  Guard guard;
  table[0] = 7;
  return table[0];
}
int operand(Table& table) {
  Guard guard;
  table[index()] = 9;
  return table[index()];
}
int main() {
  Table table;
  if (plain(table) != 7 || destroyed != 1 || calls != 2) return 1;
  if (operand(table) != 9 || destroyed != 2 || calls != 4) return 2;
  return 0;
}
