// expect exit 0
// Bare captureless lambdas initialize static scalar and array function pointers
// with invoker addresses; the invoker's local static retains lexical ABI scope.
int (*pointers[2])() = { [] { return 1; }, [] { return 2; } };
int main()
{
  static int (*next)() = [] { static int count = 0; return ++count; };
  if (next() != 1 || next() != 2) return 1;
  return pointers[0]() + pointers[1]() == 3 ? 0 : 2;
}
