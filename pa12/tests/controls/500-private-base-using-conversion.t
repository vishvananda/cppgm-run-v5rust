// expect exit 0
// A public using-declaration supplies conversion access through a private base.
struct Base { int n; Base():n(7){} operator int() const { return n; } };
struct Derived : private Base { using Base::operator int; };
int main() {
  Derived object;
  int scalar = object;
  int sum = object + 1;
  int condition = 0;
  if (Derived value = object) condition = 1;
  int selected = 0;
  switch (Derived value = object) { case 7: selected = 1; }
  return scalar != 7 || sum != 8 || condition != 1 || selected != 1;
}
