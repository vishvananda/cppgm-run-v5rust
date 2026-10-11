// N3485 focus: 4.2 [conv.array], 5.2.2 [expr.call]
int take(int *p) { return 0; }
int f() {
  int a[3];
  return take(a);
}
