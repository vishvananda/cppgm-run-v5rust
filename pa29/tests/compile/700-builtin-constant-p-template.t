// VALIDATION: compile-pass
// GNU/Clang hosted builtin focus: __builtin_constant_p lowering after template instantiation

template<class T>
int from_template(T value)
{
  return __builtin_constant_p(value) ? 1 : 0;
}

int main()
{
  return from_template(3);
}
