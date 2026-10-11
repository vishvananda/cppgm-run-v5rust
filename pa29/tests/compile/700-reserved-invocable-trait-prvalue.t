template<class...>
struct __is_invocable
{
  static const bool value = true;
  typedef bool value_type;
  constexpr operator value_type() const { return value; }
};

struct fn
{
  int operator()(int) const { return 1; }
};

static_assert(__is_invocable<fn, int>{}, "reserved trait object converts to bool");

int main()
{
  return 0;
}
