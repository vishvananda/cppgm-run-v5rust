template<class T>
struct S {
  S();
};

template<>
struct S<int> {
  S();
};

template<>
S<int>::S() {}
