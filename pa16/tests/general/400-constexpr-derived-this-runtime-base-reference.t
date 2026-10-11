// An address-only runtime object may reach a constexpr member during folding.
// If its *this result cannot materialize a base subobject, constant evaluation
// must decline so normal runtime call analysis can continue.

struct runtime_base {
  int value() const
  {
    return 1;
  }
};

struct runtime_derived : runtime_base {
  constexpr runtime_derived()
    : runtime_base()
  {}

  constexpr runtime_base const &get() const
  {
    return *this;
  }
};

int test()
{
  const runtime_derived object;
  return object.get().value();
}

int main()
{
  return test() != 1;
}
