// VALIDATION: compile-fail
// N3485 focus: 8.4.3 [dcl.fct.def.delete], 12.8 [class.copy]
// Expected: deleted move assignment must be rejected when selected.

struct S
{
  S() {}
  S & operator=(const S &) = default;
  S & operator=(S &&) = delete;
};

int main()
{
  S a;
  S b;
  a = static_cast<S &&>(b);
  return 0;
}
