// lowir.md unwind=no on call signatures: a call through a pointer, reference
// or pointer to member states unwind=no when its declaration (a variable, a
// parameter, a data member) has a non-throwing exception-specification (15.4p2,
// p12), noexcept(constant-expression) included; noexcept(call) agrees
// (5.3.7p3).
int quiet(int x) noexcept { return x + 1; }
int loud(int x) { if (x < 0) throw x; return x + 2; }

int (*quiet_pointer)(int) noexcept = quiet;
int (*loud_pointer)(int) = loud;
int (*computed_pointer)(int) noexcept(sizeof(int) == 4) = quiet;
struct Table { int (*entry)(int) noexcept; };

static_assert(noexcept(quiet_pointer(0)), "a noexcept pointer's call does not throw");
static_assert(!noexcept(loud_pointer(0)), "another pointer's call may throw");
static_assert(noexcept(computed_pointer(0)), "a true noexcept(expression) pointer's call does not throw");

int through_reference(int (&function)(int) noexcept) { return function(0); }
int through_parameter(int function(int) noexcept) { return function(0); }

int main()
{
  Table table = {quiet};
  try
  {
    return quiet_pointer(0) + loud_pointer(0) + computed_pointer(0) + table.entry(0) +
      through_reference(quiet) + through_parameter(quiet) == 7 ? 0 : 1;
  }
  catch (int)
  {
    return 2;
  }
}
