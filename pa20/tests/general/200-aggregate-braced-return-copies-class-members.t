// VALIDATION: compile-pass
// Returning a braced aggregate with nontrivial class members performs the
// supported member copy construction in declaration order.

struct member_value
{
  int * pointer;
  int local;

  member_value(int input) noexcept
    : pointer(&local), local(input)
  {
  }

  member_value(const member_value & other) noexcept
    : pointer(&local), local(other.local)
  {
  }

  member_value(member_value && other) noexcept;
};

struct aggregate_value
{
  int prefix;
  member_value first;
  member_value second;
};

aggregate_value make_aggregate(member_value first, member_value second)
{
  return {1, first, second};
}

int main()
{
  return 0;
}
