// VALIDATION: compile-pass
// N3485 focus: 10.2 [class.member.lookup], 13.3.1 [over.match.funcs]

struct Base
{
  int select(const char *, const char *, char *)
  {
    return 2;
  }

  int select(char)
  {
    return 1;
  }
};

struct Derived : Base
{
};

int main()
{
  Derived object;
  const char *first = "x";
  char out = 0;
  return object.select(first, first + 1, &out) == 2 ? 0 : 1;
}
