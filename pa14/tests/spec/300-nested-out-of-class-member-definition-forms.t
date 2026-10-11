// A class-template instantiation must retain out-of-class definitions for a
// nested class's defaulted copy constructor and conversion operator.  A
// repeated injected-class-name qualifier still denotes the original owner.
template <class T>
struct Outer {
  struct Ref {
    T& value;

    Ref(T& input) : value(input) {}
    Ref(Ref const&);
    operator bool() const;
  };
};

template <class T>
Outer<T>::Ref::Ref(Ref const&) = default;

template <class T>
Outer<T>::Ref::operator bool() const {
  return value != 0;
}

template <class T>
struct Iter {
  int value;

  Iter& operator++();
};

template <class T>
Iter<T>& Iter<T>::Iter::operator++() {
  ++value;
  return *this;
}

int main() {
  int value = 1;
  Outer<int>::Ref first(value);
  Outer<int>::Ref second(first);
  Iter<int> iter = {0};
  ++iter;
  return static_cast<bool>(second) && iter.value == 1 ? 0 : 1;
}
