struct S {};
const S * f(bool cond, S * a, const S * b) {
  return cond ? a : b;
}
int main() { return 0; }
