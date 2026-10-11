struct L {
  int v;
  bool operator==(const L& y) const { return v == y.v; }
};
int main() {
  L a;
  L b;
  a.v = 1;
  b.v = 1;
  return a == b ? 0 : 1;
}
