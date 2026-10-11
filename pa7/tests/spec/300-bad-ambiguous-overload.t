// N3485 focus: 13.3.3 [over.match.best] ambiguous overload resolution
void f(long, long) {}
void f(unsigned long, unsigned long) {}

int main() {
  short x = 1;
  f(x, x);
  return 0;
}
