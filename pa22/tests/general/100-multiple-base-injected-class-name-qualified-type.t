// VALIDATION: compile-pass
// Multiple-base lookup: the same base injected-class-name reached through more
// than one base path still denotes one type entity.

struct Base
{
  int value;
};

struct Left : Base
{
};

struct Right : Base
{
};

struct Derived : Left, Right
{
};

Derived::Base *p;

int main()
{
  return p == 0 ? 0 : 1;
}
