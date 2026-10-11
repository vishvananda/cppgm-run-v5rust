// AUDIT-ID: ATTR-NORETURN
// AUDIT-EXPECT: reject
// N3485 7.6.3p1: a block-scope variable is not a function.
int read()
{
  [[noreturn]] int value = 0;
  return value;
}
