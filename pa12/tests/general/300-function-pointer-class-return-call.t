struct S { int x; };

S make_s() {
  S s = {7};
  return s;
}

int main() {
  S (*g)() = make_s;
  return g().x - 7;
}
