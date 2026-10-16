// expect exit 0
// Synthesized definitions fill selected instance plans without demanding unused
// members. A nontrivial copy still calls its selected subobject constructor.
int constructions;
int copies;
int destructions;

struct Element {
  int value;
  Element() : value(7) { ++constructions; }
  Element(const Element& other) : value(other.value) { ++copies; }
  ~Element() { ++destructions; }
};

template<class T> struct Owner {
  T value;
  Owner();
  Owner(const Owner&);
  ~Owner();
};

template<class U> Owner<U>::Owner() = default;
template<class U> Owner<U>::Owner(const Owner&) = default;
template<class U> Owner<U>::~Owner() = default;
template struct Owner<long>;

template<class T> struct Outer {
  struct Reference {
    T& value;
    Reference(T& input) : value(input) {}
    Reference(const Reference&);
  };
};

template<class U>
Outer<U>::Reference::Reference(const Reference&) = default;

int main() {
  Owner<long> explicitly_instantiated;
  int value = 3;
  Outer<int>::Reference first(value);
  Outer<int>::Reference second(first);
  second.value = 5;
  if (value != 5) return 1;
  {
    Owner<int> a;
    a.value = 5;
    Owner<int> b(a);
    if (b.value != 5) return 5;
  }
  {
    Owner<Element> a;
    Owner<Element> b(a);
    Owner<Element> c(b);
    if (a.value.value + b.value.value + c.value.value != 21) return 2;
  }
  if (constructions != 1 || copies != 2 || destructions != 3) return 3;
  // Completing this class must leave its deleted default constructor unused.
  if (sizeof(Owner<int&>) != sizeof(int*)) return 4;
  return 0;
}
