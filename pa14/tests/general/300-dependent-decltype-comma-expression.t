template<class T>
T&& declval();

template<class T>
struct wrap {
  typedef decltype(declval<T>(), char(0)) type;
};

int main() {
  wrap<int>::type x = char(0);
  return static_cast<int>(x);
}
