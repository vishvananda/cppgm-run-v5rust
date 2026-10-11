// VALIDATION: compile-pass
// N3485 focus: 7 [dcl.dcl], 10.2 [class.member.lookup], 14.5.2 [temp.mem]

struct base
{
  template<class String>
  int fail(String const &, char const *, int)
  {
    return 7;
  }
};

struct derived : base
{
  int test()
  {
    static_assert(sizeof(int) >= 2, "");
    return fail("", "input.cpp", 1);
  }
};

int main()
{
  derived value;
  return value.test() == 7 ? 0 : 1;
}
