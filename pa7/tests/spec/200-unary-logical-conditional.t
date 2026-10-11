// N3485 focus: 5.3.1 [expr.unary.op], 5.14 [expr.log.and], 5.16 [expr.cond]
int f() {
  int x = 0;
  int y = 1;
  bool b = !x || (y && true);
  ++x;
  y++;
  return b ? x : y;
}
