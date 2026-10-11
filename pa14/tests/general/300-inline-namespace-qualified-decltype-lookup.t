namespace std {
  inline namespace __1 {
    template<class T>
    T&& __declval(int);

    template<class T>
    struct holder {
      typedef decltype(std::__declval<T>(0)) type;
    };
  }
}

std::holder<int>::type f() {
  return 0;
}

int main() {
  return 0;
}
