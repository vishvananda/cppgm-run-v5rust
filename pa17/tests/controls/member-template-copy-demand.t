// expect exit 0
// Copied member-template heads keep defaults in each enclosing specialization;
// bodies remain lazy, and a later out-of-class definition supplies renamed frames.
template<class T>
struct Outer {
  T value;

  template<class U = T>
  int sizes() { return sizeof(T) + sizeof(U); }

  template<class U>
  T add(U input);

  template<class U>
  int unused(U input) { return input.missing; }

  template<class U = T>
  struct Inner {
    int sizes() { return sizeof(T) + sizeof(U); }
  };
};

int main() {
  Outer<char> first = {2};
  Outer<long> second = {7};
  Outer<char>::Inner<> inner_first;
  Outer<long>::Inner<int> inner_second;
  return first.add(3) + second.add(4) + first.sizes<>() + first.sizes<>()
      + second.sizes<>() + inner_first.sizes() + inner_second.sizes() - 50;
}

template<class Owner>
template<class Argument>
Owner Outer<Owner>::add(Argument input) {
  return value + input;
}
