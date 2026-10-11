template<class T, class U>
struct is_same {
  static const bool value = false;
};

template<class T>
struct is_same<T, T> {
  static const bool value = true;
};

template<class T>
T&& __declval(int);

template<class T>
T __declval(long);

template<class T>
auto declval() noexcept -> decltype(__declval<T>(0)) {
  static_assert(!is_same<T, T>::value, "");
}

template<class T>
struct X {
  using type = decltype(declval<T>());
};

typedef X<int>::type int_rref;

int main() {
  return 0;
}
