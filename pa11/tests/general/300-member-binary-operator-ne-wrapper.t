struct L {
  int v;
  bool operator==(const L& y) const { return v == y.v; }
  bool operator!=(const L& y) const { return !(*this == y); }
};
int main() {
  L a;
  L b;
  a.v = 1;
  b.v = 2;
  return a != b ? 0 : 1;
}
