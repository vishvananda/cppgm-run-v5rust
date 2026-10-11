// VALIDATION: compile-fail
// N3485 focus: 8.4.2 [dcl.fct.def.default], 12.8 [class.copy]
// Expected: a defaulted move assignment can be implicitly deleted.

struct S
{
  S(int value = 0) : value(value) {}
  S & operator=(S &&) = default;

  const int value;
};

int main()
{
  S a(1);
  S b(2);
  a = static_cast<S &&>(b);
  return 0;
}
