int f(int x) {
  auto outer = [&]() {
    auto inner = [&]<class T>(T t) { return x + t; };
    return inner(2);
  };
  return outer();
}
