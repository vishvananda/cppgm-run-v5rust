template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class A, class B>
struct is_same {
  static const bool value = false;
};

template<class A>
struct is_same<A, A> {
  static const bool value = true;
};

template<class T>
struct fn {
  template<class U,
           class En = typename enable_if<is_same<T, U>::value>::type>
  int f(U) const {
    return 0;
  }

  template<class U,
           class E1 = void,
           class En = typename enable_if<
               !(is_same<T, U>::value || is_same<U, T>::value)>::type>
  int f(U) const {
    return 1;
  }
};

int main() {
  return fn<int>().f(0);
}
