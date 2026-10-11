struct X {
  int n;
  X(int x) : n(x) {}
  auto&& base() && { return static_cast<X&&>(*this); }
};

int main() {
  return X(4).base().n - 4;
}
