// expect exit 0
// Listed elements and trailing default arguments have separate unwind queries.
int calls;
int alive;
int next() { ++calls; return 7; }
struct Element {
  int value;
  Element(int n = next()) noexcept : value(n) { ++alive; }
  ~Element() { --alive; }
};
Element* make(int n) { return new Element[n]{1}; }
int main() {
  Element* p = make(3);
  if (calls != 2 || alive != 3) return 1;
  if (p[0].value != 1 || p[1].value != 7 || p[2].value != 7) return 2;
  delete[] p;
  return alive == 0 ? 0 : 3;
}
