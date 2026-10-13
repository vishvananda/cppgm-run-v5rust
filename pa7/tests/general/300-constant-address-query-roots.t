int object;
int array[3];
int function(int);
constexpr int* zero = nullptr;
int queries(int* pointer)
{
  return __builtin_constant_p(pointer) + __builtin_constant_p(array) +
         __builtin_constant_p(function) + __builtin_constant_p(&object) +
         __builtin_constant_p((int*)0) + __builtin_constant_p(zero) +
         __builtin_constant_p("abc" + 2);
}
