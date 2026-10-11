// VALIDATION: compile-fail
// C++11 permits this copy to be elided, but the selected copy constructor
// must still be accessible at the initialization site.

struct value
{
  value() {}

private:
  value(value const &);

  friend value make_value();
};

value make_value()
{
  return value();
}

int main()
{
  value result = make_value();
  (void)result;
}
