namespace N {
template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class T>
struct S {
  S();
  S(const S&);
};

template<class T, class... Args, enable_if_t<true, int> = 0>
S<T> make(Args&&... args) {
  return S<T>();
}
}

int main() {
  N::S<int> s = N::make<int>();
  return 0;
}
