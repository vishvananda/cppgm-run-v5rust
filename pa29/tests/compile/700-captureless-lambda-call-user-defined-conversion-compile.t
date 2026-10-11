struct Sink {
  template<class F>
  Sink(F) {}
};

int main() {
  auto f = [](const Sink &) {};
  int x = 0;
  auto g = [&](int) { x = 1; };
  f(g);
  return x;
}
