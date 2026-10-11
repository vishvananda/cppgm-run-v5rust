// VALIDATION: compile-fail
// N3485 focus: 12.8 [class.copy]
// Expected: an implicitly declared copy assignment is deleted when a base copy
// assignment is deleted.

struct Base
{
  Base & operator=(const Base &) = delete;
};

struct Derived : Base
{
};

int main()
{
  Derived a;
  const Derived b;
  a = b;
  return 0;
}
