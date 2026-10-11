struct S
{
  int value;
};

struct Placeholder
{};

struct DataMemberLambda
{
  int S::* member;

  int operator()(S * object) const
  {
    return object->*member;
  }
};

DataMemberLambda operator->*(Placeholder, int S::* member)
{
  DataMemberLambda result = { member };
  return result;
}

int main()
{
  S s = { 7 };
  Placeholder _1;
  return (_1->*&S::value)(&s) == 7 ? 0 : 1;
}
