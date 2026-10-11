template<class T>
struct Pair {
  T first;
  T second;
};

template<class T>
struct Tree {
  template<class... A>
  __attribute__((__visibility__("hidden")))
  int emplace(A&&... a) {
    auto [x, y] = Pair<int>{1, 2};
    return x + y + sizeof...(a);
  }
};

int main() {
  return 0;
}
