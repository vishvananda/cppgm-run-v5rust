template<class T>
struct View {
  T value;
};

template<class T, class A>
int helper(A a, View<T> v) {
  return a + v.value;
}

template<class T, class A>
struct S {
  T value;
  explicit S(T x) : value(x) {}
  operator View<T>() const {
    View<T> v;
    v.value = value;
    return v;
  }
};

int main() {
  S<int, int> s(7);
  return helper<int>(3, s) - 10;
}
