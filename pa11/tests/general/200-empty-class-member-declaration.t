struct X {
  ;
  int v;
  X() : v(7) {}
};

int main() {
  X x;
  return x.v - 7;
}
