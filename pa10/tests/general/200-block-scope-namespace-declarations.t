// N3485 3.5p6: a block-scope function declaration or extern variable
// declaration names a member of the innermost enclosing namespace, the same
// entity as that namespace's own declarations and definition, even one
// that appears only later; a block-scope declaration has no automatic
// storage of its own.
namespace N {
int first(int x) { extern int later; return x + later; }
int later = 10;
}
int helper(int);
int twice(int x)
{
  int helper(int);
  int local_function(int), value = 3;
  return helper(x) + local_function(value);
}
int address(int x)
{
  extern int scale(int);
  int (*pointer)(int) = scale;
  return pointer(x);
}
int defined_after(int x)
{
  extern int bias;
  return x + bias;
}
int bias = 7;
int helper(int x) { return x * 2; }
int local_function(int x) { return x + 1; }
int scale(int x) { return x * 3; }
int main()
{
  return N::first(1) == 11 && twice(5) == 14 && address(2) == 6 &&
    defined_after(1) == 8 ? 0 : 1;
}
