// N3485 focus: 6.4.1 [stmt.if] if statement
int g(int x) { return x; }
int f() {
  if (g(0) < 1)
    return 1;
  else
    return 2;
}
