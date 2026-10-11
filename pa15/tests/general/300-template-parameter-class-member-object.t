template<typename T>
struct Iter;

template<>
struct Iter<int> {
  int* p;
  Iter() : p(0) {}
};

template<typename T>
struct R {
  T t;
};

using X = R<Iter<int> >;

long f() {
  X x;
  return x.t.p == 0;
}
