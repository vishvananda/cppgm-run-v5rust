// VALIDATION: compile-pass
// A defaulted constructor includes a member constructor selected through a
// default argument when computing its implicit exception specification.

struct member
{
  member(int = 0)
  {
  }
};

struct owner
{
  member value;
  owner() = default;
};

static_assert(!noexcept(owner()), "member construction may throw");

int main()
{
  return noexcept(owner()) ? 1 : 0;
}
