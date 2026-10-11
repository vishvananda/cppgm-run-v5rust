// VALIDATION: compile-fail
// N3485 focus: 7.2 [dcl.enum]
// Expected: scoped enumerations do not convert implicitly to int.

enum class Mode : int
{
  A = 1
};

int takes_int(int);

int main()
{
  return takes_int(Mode::A);
}
