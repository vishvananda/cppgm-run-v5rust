struct S { int *p; };

int main() {
  int const * const S::* cpm = &S::p;
  (void)cpm;
  return 0;
}
