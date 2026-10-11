// VALIDATION: compile-fail
// Reference binding preserves an existing object's identity, but it must not
// invent persistent storage for a temporary argument.

constexpr const int *address_of(const int &value)
{
  return &value;
}

constexpr const int *address = address_of(5);

int main()
{
  return 0;
}
