constexpr int one_argument(int value) noexcept
{
  return value;
}

constexpr int call_pointer(int (*function)(int), int value)
{
  return function(value);
}

constexpr int call_reference(int (&function)(int), int value)
{
  return function(value);
}

static_assert(call_pointer(&one_argument, 2) == 2,
              "constexpr function pointer call");
static_assert(call_reference(one_argument, 3) == 3,
              "constexpr function reference call");

int main()
{
  return 0;
}
