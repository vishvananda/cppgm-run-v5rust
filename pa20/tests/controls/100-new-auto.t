// N3485 5.3.4p2, 7.1.6.4p4: auto in a new-type-id: the allocated type is
// deduced from the new-initializer ( assignment-expression ) as a variable's,
// with the new-type-id's declarator operators and cv-qualifiers. expect exit 0
int y = 3;
int main() {
  auto p = new auto(9);
  int* q = &y;
  auto** pp = new auto*(q);
  const int* c = new const auto(y);
  long* l = new auto(5L);
  int ok = *p == 9 && **pp == 3 && *c == 3 && *l == 5;
  delete p; delete pp; delete c; delete l;
  return ok ? 0 : 1;
}
