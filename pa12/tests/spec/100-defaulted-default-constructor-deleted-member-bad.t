// VALIDATION: compile-fail
// N3485 focus: 8.4.2 [dcl.fct.def.default], 12.1 [class.ctor]
// Expected: a defaulted default constructor is deleted when a member cannot be default-constructed.

struct needs_arg
{
  explicit needs_arg(int);
};

struct holder
{
  needs_arg value;
  holder() = default;
};

int main()
{
  holder value;
  return 0;
}
