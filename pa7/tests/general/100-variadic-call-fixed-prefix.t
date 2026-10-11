// HHC-112
extern "C" void f(const char *, ...);

void g() {
  f("x");
}

int main() {
  return 0;
}
