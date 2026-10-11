struct Derived;

struct Base
{
  Base() : tag(0) {}
  Base(Derived &) : tag(7) {}
  Base(const Base &) = delete;
  int tag;
};

struct Derived : Base
{
  Derived() : Base() {}
};

int main()
{
  Derived d;
  Base b(d);
  return b.tag == 7 ? 0 : 1;
}

// VALIDATION: compile-pass
