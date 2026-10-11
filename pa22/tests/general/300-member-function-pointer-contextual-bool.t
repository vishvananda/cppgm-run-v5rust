struct member_pointer_contextual_bool
{
  int value;
  int f()
  {
    return 7;
  }
};

static_assert(&member_pointer_contextual_bool::value, "nonnull data member");
static_assert(&member_pointer_contextual_bool::f, "nonnull member function");
static_assert(!static_cast<int member_pointer_contextual_bool::*>(nullptr),
              "null data member");
static_assert(!static_cast<int (member_pointer_contextual_bool::*)()>(nullptr),
              "null member function");

int main()
{
  int (member_pointer_contextual_bool::*member)() =
    &member_pointer_contextual_bool::f;
  if(!member) {
    return 1;
  }

  member = 0;
  if(member) {
    return 2;
  }
  return 0;
}
