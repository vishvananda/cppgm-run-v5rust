// lowir.md elision=copy: the permission requires a temporary of the object's
// own class (12.8p31); copy-initializing a base object from a derived class's
// prvalue conditional slices it and states none.
struct Base
{
  int id;
  explicit Base(int x) : id(x) {}
  Base(const Base& other) : id(other.id) {}
  ~Base() {}
};
struct Derived : Base
{
  int extra;
  explicit Derived(int x) : Base(x), extra(x) {}
};

int main()
{
  bool c = true;
  Base same = c ? Base(1) : Base(2);
  Base sliced = c ? Derived(3) : Derived(4);
  return same.id + sliced.id == 4 ? 0 : 1;
}
