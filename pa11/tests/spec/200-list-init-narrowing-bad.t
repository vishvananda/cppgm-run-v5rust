// VALIDATION: compile-fail
// N3485 focus: 8.5.4 [dcl.init.list]
// Expected: narrowing in list-initialization must be rejected.

int main()
{
  int value{1.5};
  return value;
}
