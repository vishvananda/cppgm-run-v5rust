struct S {
  int x;
};

struct R {
  S& ref;

  R(S& x) : ref(x) {}
};

int main() {
  S s;
  s.x = 3;
  R r(s);
  S* candidate = nullptr;
  S& decl_scope = candidate ? *candidate : r.ref;
  decl_scope.x = 9;
  return s.x == 9 ? 0 : 1;
}
