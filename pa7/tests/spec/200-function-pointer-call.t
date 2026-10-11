// N3485 focus: 5.2.2 [expr.call] function pointer call
int f() {
  int (*fp)(int);
  return fp(0);
}
