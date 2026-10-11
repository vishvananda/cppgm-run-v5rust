template<class... T>
struct outer {
  struct inner;
};

template<class T>
struct outer<T> {
  typedef int inner;
};

template<class... T>
struct outer<T...>::inner {
  bool operator==(inner const& other) const;
};

template<class... T>
bool outer<T...>::inner::operator==(inner const& other) const {
  return this == &other;
}

int main() {
  outer<int, long>::inner value;
  return value == value ? 0 : 1;
}
