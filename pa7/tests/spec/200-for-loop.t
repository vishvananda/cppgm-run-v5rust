// N3485 focus: 6.5.3 [stmt.for] for statement
int f() {
  int sum = 0;
  for (int i = 0; i < 3; i = i + 1) {
    sum = sum + i;
  }
  return sum;
}
