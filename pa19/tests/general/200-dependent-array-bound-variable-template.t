template<class T>
inline const unsigned long size_v = sizeof(T);

template<class T>
struct Pad {
  char bytes[size_v<T>];
};

template<class D>
struct Outer {
  typedef typename D::pointer pointer;
  Pad<pointer> p;
};

template<class D>
void f(Outer<D> & x) {}

int main() {
  return 0;
}
