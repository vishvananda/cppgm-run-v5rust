int classify(int value)
{
  return __builtin_constant_p(value + 1) +
         __builtin_constant_p(true ? 2 : value);
}
