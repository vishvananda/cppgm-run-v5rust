// N3485 12.3.2, 4.10p3: a scalar conversion result's base projection
// belongs to the returned pointer, independently of its receiver's base.
// expect exit 0
struct Base { int n; };
struct Derived : Base { int m; };
struct Source {
  Derived* p;
  operator Derived*() { return p; }
};
struct Box : Source {};
struct ReferenceSource {
  Derived* p;
  operator Derived*&() { return p; }
};
Base* returned(Box& x) { return x; }
Base* returned_reference(ReferenceSource& x) { return x; }
int take(Base* p) { return p->n; }
int main() {
  Derived object;
  object.n = 7;
  Box x;
  x.p = &object;
  ReferenceSource y;
  y.p = &object;
  Base* p = x;
  Base* q = y;
  if (p != &object || q != &object) return 1;
  if (returned(x) != p || returned_reference(y) != p) return 2;
  if (take(x) != 7 || take(y) != 7) return 3;
  x.p = nullptr;
  y.p = nullptr;
  if (returned(x) != nullptr || returned_reference(y) != nullptr) return 4;
  return 0;
}
