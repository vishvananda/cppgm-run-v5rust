// VALIDATION: compile-pass
// N3485 focus: 13.3.3.2 [over.ics.rank]. When otherwise indistinguishable
// implicit-object sequences bind a const rvalue, the const-rvalue-qualified
// member is preferred over the const-lvalue-qualified member.

struct RefPick
{
  int pick() const &
  {
    return 1;
  }

  int pick() const &&
  {
    return 2;
  }
};

int main()
{
  const RefPick value;
  return static_cast<const RefPick &&>(value).pick() == 2 ? 0 : 1;
}
