// VALIDATION: compile-fail
// N3485 focus: 14.8.2.4 [temp.deduct.partial], 13.3 [over.match]
// Expected: cv-symmetric pointer partial ordering remains ambiguous.

template<typename T>
int pick(const T *)
{
  return 1;
}

template<typename T>
int pick(volatile T *)
{
  return 2;
}

int main()
{
  const volatile int * value = 0;
  return pick(value);
}
