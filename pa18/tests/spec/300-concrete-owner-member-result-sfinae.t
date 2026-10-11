// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], 14.8.2.8 [temp.deduct.type]
// A concrete class-template owner makes complementary member result types SFINAE.

template<bool Condition, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class T>
struct is_movable {
  static const bool value = true;
};

template<class T, class E>
struct result {
  template<class U = T>
  typename enable_if<is_movable<U>::value, T>::type
  value() && {
    return 7;
  }

  template<class U = T>
  typename enable_if<!is_movable<U>::value, T &&>::type
  value() &&;
};

template<class T, class E>
T read(result<T, E> && input) {
  return static_cast<result<T, E> &&>(input).value();
}

int main() {
  return read(result<int, void>()) == 7 ? 0 : 1;
}
