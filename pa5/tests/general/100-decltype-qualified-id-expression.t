// HHC-125
template<class T>
struct B { static const bool value = true; };

template<class T>
B<T> f();

template<class T>
struct X {
  static const bool value = decltype(f<T>())::value;
};

int main() { return X<int>::value ? 0 : 1; }
