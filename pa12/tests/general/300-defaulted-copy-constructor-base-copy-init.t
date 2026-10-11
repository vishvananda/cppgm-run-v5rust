struct Base {
  int x;

  Base(int value) : x(value) {}
  Base(const Base&) = default;
};

struct Derived : Base {
  Derived(int value) : Base(value) {}
  Derived(const Derived&) = default;
};

int main()
{
  Derived source(7);
  Derived copy(source);
  return copy.x;
}
