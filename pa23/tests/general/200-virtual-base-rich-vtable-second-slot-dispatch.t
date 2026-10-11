// VALIDATION: compile-pass
// A virtual call through a class with a virtual base selects the requested
// logical slot when each vtable function row also carries an adjustment.

struct root
{
  virtual int anchor() { return 1; }
};

struct derived : virtual root
{
  virtual int first() { return 2; }
  virtual int second() { return 3; }
};

int call_second(derived & value)
{
  return value.second();
}
