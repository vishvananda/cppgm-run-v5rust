// N3485 14.3, 14.5.3: canonical pack arguments, frame binding and forwarding;
// expect exit 0
template<class T = int, class... U>
struct types {
  static const unsigned long count = sizeof...(U);
  static unsigned long length() { return sizeof...(U); }
  void unused() { T::missing; }
};

template<class T, T... N>
struct values {
  static const unsigned long count = sizeof...(N);
};

template<class... U>
struct forward {
  typedef types<int, U*...> type;
};

template<int... N>
struct forward_values {
  typedef values<int, N...> type;
};

template<class A, class B>
struct fixed_pair {};

template<class... U>
struct forward_fixed {
  typedef fixed_pair<U...> type;
};

template<class... U>
struct bound_owner {
  template<class... V>
  struct inner {
    typedef types<int, fixed_pair<U, V>...> type;
  };
};

template<class... U>
struct tuple {};

template<class A, class B>
int cardinality(tuple<A, B>) { return 2; }

static_assert(types<>::count == 0, "empty defaulted pack");
static_assert(types<int, char, long>::count == 2, "type pack");
static_assert(values<int, 1, 2L, 3>::count == 3, "converted value pack");
static_assert(forward<char, long>::type::count == 2, "compound expansion");
static_assert(forward<>::type::count == 0, "empty expansion");
static_assert(forward_values<1, 2>::type::count == 2, "value forwarding");
static_assert(sizeof(forward_fixed<int, char>::type) == 1, "fixed slots");
static_assert(bound_owner<int, long>::inner<char, float>::type::count == 2,
              "bound packs expand in lockstep");

int main() {
  tuple<int, long> value;
  return types<int, char, long>::length() == 2 && cardinality(value) == 2 ? 0 : 1;
}
