// VALIDATION: compile-pass
// N3485 focus: 9.5 [class.union] block-scope anonymous-union member injection

int read_variant()
{
  union
  {
    int value;
    long long bits;
  };

  value = 7;
  return value;
}

int main()
{
  return read_variant() == 7 ? 0 : 1;
}
