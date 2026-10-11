struct Outer {
  struct Hidden* p;

  void set(struct Hidden* q) {
    p = q;
  }
};

int main() {
  Outer o;
  o.set(0);
  return o.p == 0 ? 0 : 1;
}
