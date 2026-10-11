// N3485 focus: 14.8.2.2 [temp.deduct.funcaddr]
// A target function type participates in template argument deduction even
// when the template parameter occurs only in the function return type.

template<class T>
T make_value()
{
  return T();
}

typedef int (*maker)();

maker selected = make_value;

int main()
{
  return selected();
}
