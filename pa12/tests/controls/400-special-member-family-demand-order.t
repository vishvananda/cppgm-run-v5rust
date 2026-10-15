// N3485 12.8: copy/move families retain base-entry transfers; expect exit 0
int copies;
int moves;
int copy_assignments;
int move_assignments;
struct Value
{
  int n;
  Value() : n(37) {}
  Value(const Value& other) : n(other.n) { ++copies; }
  Value(Value&& other) : n(other.n) { ++moves; other.n = 0; }
  Value& operator=(const Value& other)
  {
    n = other.n;
    ++copy_assignments;
    return *this;
  }
  Value& operator=(Value&& other)
  {
    n = other.n;
    other.n = 0;
    ++move_assignments;
    return *this;
  }
};
struct Derived : Value {};
int main()
{
  Derived a;
  Derived b = static_cast<Derived&&>(a);
  Derived c(b);
  a = static_cast<Derived&&>(b);
  b = c;
  return a.n != 37 || b.n != 37 || c.n != 37
    || copies != 1 || moves != 1 || copy_assignments != 1 || move_assignments != 1;
}
