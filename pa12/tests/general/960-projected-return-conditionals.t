// A return copies each selected glvalue through the whole choice's base binding.
// A nested choice selects one address inside the constructor's call region.
int live;
struct Base {
  Base() { ++live; }
  Base(const Base&) { ++live; }
  ~Base() { --live; }
};
struct Derived : Base {
  Derived() {}
  ~Derived() {}
};
Base flat(bool which) {
  Derived left, right;
  return which ? left : right;
}
Base nested(bool outer, bool inner) {
  Derived left, right;
  return outer ? (inner ? left : right) : right;
}
int main() {
  Base first = flat(true);
  Base second = nested(true, false);
  return live != 2;
}
