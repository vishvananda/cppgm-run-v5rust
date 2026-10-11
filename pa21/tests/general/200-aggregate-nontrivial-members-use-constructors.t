struct Member
{
  int * pointer;
  int local;

  Member(int value) noexcept : pointer(&local), local(value)
  {
  }

  Member(const Member & other) noexcept
    : pointer(&local), local(other.local)
  {
  }

  ~Member() noexcept
  {
  }
};

struct Aggregate
{
  int prefix;
  Member first;
  Member second;
};

Aggregate make_aggregate()
{
  return { 1, Member(2), Member(3) };
}

int main()
{
  Aggregate value = make_aggregate();
  return value.prefix == 1 &&
         value.first.local == 2 &&
         value.first.pointer == &value.first.local &&
         value.second.local == 3 &&
         value.second.pointer == &value.second.local ? 0 : 1;
}
