template<int N> struct step {
  template<class T>
  auto operator()(T value) -> decltype(step<N - 1>()(value));
};

template<> struct step<0> {
  int operator()(int);
};

typedef decltype(step<9>()(0)) result;
