// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem]

struct add_one
{
  int base;

  int operator()(int value) const
  {
    return base + value;
  }
};

template<class Value>
struct box
{
  template<class Operation>
  int apply(Operation operation);
};

box<int> early_specialization;

int call_apply(int value)
{
  box<int> object;
  add_one operation = { value };
  return object.apply(operation);
}

template<class Value>
template<class Operation>
int box<Value>::apply(Operation operation)
{
  return operation(1);
}

int main()
{
  return call_apply(6) - 7;
}
