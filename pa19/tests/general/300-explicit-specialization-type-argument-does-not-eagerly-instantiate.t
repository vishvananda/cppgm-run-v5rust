namespace N {
  template<class Tag, class Op, class... Args>
  inline const bool flag = false;

  template<class T = void>
  struct plus {
    T operator()(const T& x, const T& y) const { return x + y; }
  };

  template<>
  struct plus<void> {
    template<class T1, class T2>
    auto operator()(T1&& t, T2&& u) const -> decltype(t + u) {
      return t + u;
    }
  };

  template<class T, class U>
  inline const bool flag<int, plus<void>, T, U> = true;
}

int main() { return N::flag<int, N::plus<void>, int, int> ? 0 : 1; }
