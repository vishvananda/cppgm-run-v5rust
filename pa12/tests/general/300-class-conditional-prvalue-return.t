struct S {
  int x;
  S(int v) : x(v) {}
};

S make(bool b) {
  return b ? S(1) : S(2);
}

int main() {
  return make(true).x - 1;
}
