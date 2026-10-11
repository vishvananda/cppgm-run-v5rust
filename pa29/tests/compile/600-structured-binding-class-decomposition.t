struct Pair {
  int first;
  int& second;
};

Pair make_pair(int& x) {
  return Pair{1, x};
}

int f() {
  int x = 0;
  auto [a, b] = make_pair(x);
  b = 7;
  return x != 7;
}

int main() { return f(); }
