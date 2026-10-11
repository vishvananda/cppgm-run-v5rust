// N3485 focus: 6.4 [stmt.select] condition declaration scope
int f() { return 1; }
int g() {
  if (int x = f())
    return x;
  return 0;
}
