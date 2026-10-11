// A scalar object passed through a constexpr reference parameter must retain
// its storage identity so taking the parameter's address remains constant.

constexpr const int *address_of(const int &value)
{
  return &value;
}

static const int stored = 5;
constexpr const int *address = address_of(stored);

static_assert(address == &stored, "reference must preserve object identity");

int main()
{
  return 0;
}
