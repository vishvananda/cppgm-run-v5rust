struct Action
{
  int value;
};

struct Ref
{
  int base;
};

int operator->*(Ref & ref, Action & action)
{
  return ref.base + action.value;
}

int main()
{
  Ref ref;
  Action action;
  ref.base = 7;
  action.value = 5;
  return (ref->*action) == 12 ? 0 : 1;
}
