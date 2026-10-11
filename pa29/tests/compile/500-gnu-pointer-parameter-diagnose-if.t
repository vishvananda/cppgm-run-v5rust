void f(const char* __attribute__((__diagnose_if__(1, "x", "warning"))) s);

int main() {
  f("ok");
  return 0;
}
