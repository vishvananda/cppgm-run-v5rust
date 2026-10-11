template<int N>
struct placeholder {};

template<class T>
struct is_placeholder {
  static const int value = 0;
};

template<int N>
struct is_placeholder<placeholder<N> > {
  static const int value = N;
};

template<bool IsPlaceholder, class Ti, class Uj>
struct mu_return2 {};

template<class Ti, class Uj>
struct mu_return2<true, Ti, Uj> {
  typedef Uj& type;
};

template<class Ti, class Uj>
typename mu_return2<0<is_placeholder<Ti>::value, Ti, Uj>::type
mu(Ti&, Uj& value) {
  return value;
}

int test() {
  placeholder<1> p;
  int value = 7;
  int& result = mu(p, value);
  return result;
}

int main() {
  return test() == 7 ? 0 : 1;
}
