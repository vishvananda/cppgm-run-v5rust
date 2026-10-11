struct S {
  int x;
  S() : x(7) {}
  S(const S& o) : x(o.x) {}
};

S f(bool b) {
  S s;
  S t;
  t.x = 9;
  if(b) {
    return s;
  }
  return t;
}

int main() {
  S s = f(true);
  return s.x == 7 ? 0 : 1;
}
