// expect exit 0
// Array arguments and aggregate-helper arguments end with their full expression.
int order;
struct Element {
  int value;
  Element(int n) : value(n) {}
  ~Element() { order=order*10+value; }
};
struct References { const Element (&array)[2]; const Element& last; };
const References& references={{Element(1),Element(2)},Element(3)};
int read(const Element (&array)[2]) { return array[0].value+array[1].value; }
int main() {
  if (order!=321) return 1;
  order=0;
  int sum=read({Element(4),Element(5)});
  return sum==9 && order==54 ? 0 : 2;
}
