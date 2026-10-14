// expect exit 0
struct Base {
  int select(int) { return 1; }
};
struct Derived : Base {
  using Base::select;
  int select(int) && { return 2; }
};
int main() {
  Derived object;
  return object.select(0) == 1 ? 0 : 1;
}
