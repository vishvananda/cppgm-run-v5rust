template<class T>
struct Pair {
  T first;
  T second;
};

template<class T, int N>
int probe(Pair<T> (&arr)[N]) {
  for(auto [x, y] : arr) {
    return x + y;
  }
  return 0;
}

int main() {
  return 0;
}
