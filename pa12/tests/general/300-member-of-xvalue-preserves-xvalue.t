struct Member
{
  int select() &
  {
    return 1;
  }

  int select() &&
  {
    return 2;
  }
};

struct Value
{
  Member member;
};

int main()
{
  Value value;
  return static_cast<Value&&>(value).member.select() == 2 ? 0 : 1;
}
