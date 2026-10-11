// lowir.md unwind=no, inline_hint: a lambda's call operator is an inline
// function (5.1.2p5), non-throwing by its own noexcept, and so is the static
// function its conversion to a function pointer returns; a lambda-expression
// called directly states its call operator's unwind=no on the call signature.
int may_throw(int x);
int may_throw(int x) { return x; }

int main()
{
  auto quiet = [](int x) noexcept { return x + 1; };
  auto loud = [](int x) { return may_throw(x) + 2; };
  int (*quiet_pointer)(int) = quiet;
  int (*loud_pointer)(int) = loud;
  int direct = [](int x) noexcept { return x + 3; }(0) + [](int x) { return x + 4; }(0);
  return quiet(0) + loud(0) + quiet_pointer(0) + loud_pointer(0) + direct == 13 ? 0 : 1;
}
