struct Base {
  using T = int;
};

template<class T>
struct Derived : Base {
  using Base::T;
};
