// N3485 focus: 6.5.2 [stmt.do] do statement
// HHC-167
int f(int n) {
  do {
    --n;
  } while (n);
  return n;
}
