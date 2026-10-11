// N3485 focus: 5.17 [expr.ass] compound assignment requires modifiable lvalue
int f() {
  int x = 1;
  (x + 1) += 2;
  return x;
}
