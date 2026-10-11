// VALIDATION: compile-fail
// N3485 focus: 4.11 [conv.mem]
// Expected: pointer-to-member conversion is allowed from base to derived, not reverse.

struct Base
{
  int member;
};

struct Derived : Base
{
};

int main()
{
  int Derived::* derived_ptr = &Derived::member;
  int Base::* base_ptr = derived_ptr;
  return base_ptr != 0;
}
