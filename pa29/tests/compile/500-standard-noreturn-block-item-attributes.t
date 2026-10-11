// N3485 7.6.1p2, 7.6.3p1: leading attributes of a block item appertain to
// the declaration it introduces; an attribute other than noreturn on a
// block-scope variable is accepted, and noreturn on a function's
// declarator-id still marks the call that ends the path.
[[noreturn]] void fail();

int checked(int value)
{
  [[gnu::unused]] int spare = value;
  if (value < 0) fail();
  return value;
}
