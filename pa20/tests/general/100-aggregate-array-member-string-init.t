struct S {
  char elems[4];
};

int main() {
  S s = { "abc" };
  return s.elems[2] == 'c' && s.elems[3] == 0 ? 0 : 1;
}
