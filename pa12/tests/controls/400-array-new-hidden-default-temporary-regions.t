// expect exit 0
// A prefix temporary's segment also brackets the trailing constructor loop.
int temporaries;
int elements;
struct Temporary {
  Temporary() noexcept { ++temporaries; }
  ~Temporary() noexcept { --temporaries; }
};
struct Element {
  Element(const Temporary& = Temporary()) noexcept { ++elements; }
  ~Element() { --elements; }
};
Element* make(int n) { return new Element[n]{{}}; }
int main() {
  Element* p = make(3);
  if (temporaries != 0 || elements != 3) return 1;
  delete[] p;
  return temporaries == 0 && elements == 0 ? 0 : 2;
}
