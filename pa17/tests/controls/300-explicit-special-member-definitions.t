// expect exit 13
// Named instantiations select the existing constructor, destructor and
// conversion declarations, including a constructor template specialization.
template<class T>
struct box {
  T value;
  template<class U> box(U input);
  ~box();
  operator T() const;
};

template<class T> template<class U>
box<T>::box(U input) : value(input) {}
template<class T> box<T>::~box() {}
template<class T> box<T>::operator T() const { return value; }

template box<int>::box(char);
template box<int>::~box();
template box<int>::operator int() const;

int main() {
  box<int> value(char(13));
  return value;
}
