template<class T>
struct Base {
  int value;
};

template<class T>
struct Derived : Base<T> {
  Derived & assign_from(const Derived & other) {
    Base<T>::operator=(other);
    return *this;
  }
};

int main() {
  Derived<int> a;
  Derived<int> b;
  b.value = 12;
  a.assign_from(b);
  return a.value - 12;
}
