template<class T>
struct CtorBox {
  int value;

  CtorBox(int v = 3);
  int read() const { return value; }
};

template<class T>
CtorBox<T>::CtorBox(int v) : value(v + 10) {}

template struct CtorBox<int>;
