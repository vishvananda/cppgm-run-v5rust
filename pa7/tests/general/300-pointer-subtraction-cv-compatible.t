long f() {
  int a[3];
  int* q = a;
  const int* p = a + 2;
  return p - q;
}
