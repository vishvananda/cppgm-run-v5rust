// expect exit 0
// Copied nested template heads share syntax but form default argument facts
// in each enclosing specialization, for inline and renamed outside definitions.
struct Small { static char result(int); };
struct Large { static long result(int); };

template<class T> struct Owner {
  template<class U, class V = decltype(T::result(U()))>
  struct Value { V value; };
  template<class U> Value<U> inside(U) { return Value<U>(); }
  template<class U> Value<U> outside(U);
};
template<class Renamed> template<class Argument>
typename Owner<Renamed>::template Value<Argument>
Owner<Renamed>::outside(Argument) { return Value<Argument>(); }

template<class T> struct Arrays {
  template<class U, class V = int[sizeof(T::result(U()))]>
  struct Value { V values; };
  template<class U> Value<U> make(U) { return Value<U>(); }
};

int main() {
  Owner<Small> small;
  Owner<Large> large;
  if (sizeof(small.inside(1)) + sizeof(large.inside(1)) != 9) return 1;
  if (sizeof(large.outside(1)) + sizeof(small.outside(1)) != 9) return 2;
  if (small.inside(1).value != 0 || large.outside(1).value != 0) return 3;
  if (sizeof(small.inside(1)) != 1) return 4;
  Arrays<Small> first;
  Arrays<Large> second;
  if (sizeof(first.make(1)) + sizeof(second.make(1)) != 36) return 5;
  return 0;
}
