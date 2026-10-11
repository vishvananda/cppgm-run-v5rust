struct U {
  int value;
  U(int x) : value(x) {}
};

int get(U& u) {
  return [](U& x) { return x.value; }(u);
}

int main() {
  U u(7);
  return get(u) - 7;
}
