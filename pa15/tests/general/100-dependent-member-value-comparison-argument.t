template<int N>
struct value {
  static const int num = N;
};

template<bool B>
struct use {};

template<class T>
int f() {
  typedef use<T::num == 1> result;
  return 0;
}

int main() {
  return f<value<1> >();
}
