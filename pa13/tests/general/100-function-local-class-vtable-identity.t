// Function-local classes with the same spelling in different scopes are
// distinct types and must not share the compiler's vtable bookkeeping key.
struct base
{
  virtual int value() const { return 0; }
};

int first()
{
  struct local : base
  {
    int value() const { return 1; }
  } object;
  base & polymorphic = object;
  return polymorphic.value();
}

int second()
{
  struct local : base
  {
    int value() const { return 2; }
  } object;
  base & polymorphic = object;
  return polymorphic.value();
}

int main()
{
  return first() == 1 && second() == 2 ? 0 : 1;
}
