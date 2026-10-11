int main() {
  int n = 0;
  auto f = [&n]() { return [&n]() { return n; }; };
  return f()();
}
