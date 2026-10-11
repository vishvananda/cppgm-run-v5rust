// A captureless closure's conversion yields a function that calls its
// operator() (N3485 5.1.2/6): both run the one static variable.
int bump()
{
  auto next = []() -> int {
    static int count = 0;
    count = count + 1;
    return count;
  };
  int (*pointer)() = next;
  const int first = next();
  const int second = pointer();
  const int third = next();
  return first * 100 + second * 10 + third;
}

inline int bump_inline()
{
  auto next = []() -> int {
    static int count = 0;
    count = count + 1;
    return count;
  };
  int (*pointer)() = next;
  const int first = pointer();
  const int second = next();
  const int third = pointer();
  return first * 100 + second * 10 + third;
}

int main()
{
  if (bump() != 123) return 10;
  return bump_inline() == 123 ? 0 : 11;
}
