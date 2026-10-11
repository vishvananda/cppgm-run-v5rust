class value_init_nontrivial_member
{
  int value;

public:
  value_init_nontrivial_member() : value(42)
  {
  }

  int ok() const
  {
    return value == 42;
  }
};

struct value_init_aggregate_with_int
{
  value_init_nontrivial_member first;
  int second;
};

class value_init_aggregate_wrapper
{
  value_init_aggregate_with_int member;

public:
  value_init_aggregate_wrapper() : member()
  {
  }

  int check() const
  {
    return member.first.ok() && member.second == 0;
  }
};

void value_init_dirty_stack()
{
  int data[64];
  for(int i = 0; i != 64; ++i)
  {
    data[i] = 0x11111111;
  }
}

int main()
{
  value_init_dirty_stack();
  value_init_aggregate_wrapper value;
  return value.check() ? 0 : 1;
}
