// VALIDATION: emit-lowir
// N3485 focus: 10.3 [class.virtual]

struct KeyFunctionBase
{
  virtual ~KeyFunctionBase() {}
  virtual int value() = 0;
};

struct KeyFunctionDerived : KeyFunctionBase
{
  int value();
};

int KeyFunctionDerived::value()
{
  return 11;
}
