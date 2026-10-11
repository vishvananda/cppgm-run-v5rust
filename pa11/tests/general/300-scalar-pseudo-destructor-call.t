typedef int I;

void f(I* p) { p->~I(); }

int main() {
  I x = 0;
  f(&x);
  return 0;
}
