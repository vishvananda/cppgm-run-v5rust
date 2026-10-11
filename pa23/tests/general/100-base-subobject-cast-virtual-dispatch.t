// VALIDATION: compile-pass
// N3485 focus: 10.3 [class.virtual]

struct base
{
  virtual int value() const noexcept
  {
    return 1;
  }
};

struct derived : base
{
  int value() const noexcept override
  {
    return 7;
  }
};

int main()
{
  derived object;
  return static_cast<base const &>(object).value() == 7 ? 0 : 1;
}
