template<class T> struct Wrap { T val; };

Wrap<int> make() {
  Wrap<int> w;
  w.val = 42;
  return w;
}

int main() {
  return make().val - 42;
}
