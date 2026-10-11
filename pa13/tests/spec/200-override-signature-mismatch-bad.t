// VALIDATION: compile-fail
// N3485 focus: 10.3 [class.virtual]
// Expected: `override` requires a matching base virtual signature.

struct Base
{
  virtual int f(int)
  {
    return 0;
  }
};

struct Derived : Base
{
  int f(double) override
  {
    return 1;
  }
};

int main()
{
  return 0;
}
