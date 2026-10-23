// expect exit 0
// Each array temporary owns its whole-array prefix and nested row prefixes.
int order;
struct Element {
  int value;
  Element(int n) : value(n) {}
  ~Element() { order=order*10+value; }
};
int read(const Element (&array)[2][2]) {
  return array[0][0].value+array[1][1].value;
}
int main() {
  {
    const Element (&array)[2][2]={{Element(1),Element(2)},{Element(3),Element(4)}};
    if (read(array)!=5 || order) return 1;
  }
  if (order!=4321) return 2;
  order=0;
  int result=read({{Element(1),Element(2)},{Element(3),Element(4)}});
  return result==5 && order==4321 ? 0 : 3;
}
