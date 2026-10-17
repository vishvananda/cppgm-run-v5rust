// expect exit 0
// An extern declaration must not demand a body or an unused default argument.
template<class T>
struct box {
  box(int = T::missing) { T::missing; }
  ~box() { T::missing; }
  operator T() { return T::missing; }
};

extern template box<int>::box(int);
extern template box<int>::~box();
extern template box<int>::operator int();
extern template box<int>::box(int);

int main() { return 0; }
