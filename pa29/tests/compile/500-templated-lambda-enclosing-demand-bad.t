int main() {
  int x = 1;
  auto outer = [&]() {
    auto inner = [&]<class T>(T t) { return x + t; };
    return inner(2);
  };
  return outer();
}
