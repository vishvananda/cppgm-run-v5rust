int called;
void f() { called = 1; }
int main() {
  try { throw f; }
  catch(void (*p)()) { p(); }
  return !called;
}
