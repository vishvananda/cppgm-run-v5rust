// VALIDATION: compile-pass

struct token
{
};

struct property
{
  constexpr property() : value(0) {}
  constexpr property(token) : value(1) {}

  static constexpr property required()
  {
    return token();
  }

  friend constexpr bool operator==(property lhs, property rhs)
  {
    return lhs.value == rhs.value;
  }

  int value;
};

constexpr token query()
{
  return token();
}

constexpr int converted_value()
{
  constexpr property converted = property::required();
  return converted.value;
}

static_assert(converted_value() == 1, "typed initializer conversion");
static_assert(property::required() == token(), "return conversion");
static_assert(property::required() == query(), "operator conversion");

int main()
{
  return 0;
}
