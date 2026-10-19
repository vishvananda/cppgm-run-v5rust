struct Base { Base(); virtual ~Base(); };
struct Derived : Base {};
void first() {
  Base object;
  auto cast = [&]() { return dynamic_cast<Derived*>(&object); };
  cast();
}
