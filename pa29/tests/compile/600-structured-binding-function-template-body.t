template<class T>
struct Pair {
  T first;
  T second;
};

template<class T>
int probe(T) {
  auto [x, y] = Pair<int>{1, 2};
  return x + y;
}

int main() {
  return 0;
}
