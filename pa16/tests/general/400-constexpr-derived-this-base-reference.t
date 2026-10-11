// A constexpr method returning *this through a base reference must retain the
// current typed object and select its base subobject without reconstructing it.

struct empty_base {
  constexpr int value() const
  {
    return 1;
  }
};

struct init_tag {};

struct derived_value : empty_base {
  constexpr derived_value(init_tag)
    : empty_base()
  {}

  constexpr empty_base const &get() const
  {
    return *this;
  }
};

void test()
{
  constexpr derived_value object = init_tag();
  constexpr int result = object.get().value();
  static_assert(result == 1, "base reference must select the base subobject");
}

int main()
{
  test();
  return 0;
}
