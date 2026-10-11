int target(int& value)
{
  return value;
}

int invoke(int& value)
{
  typedef int(func_t)(int&);
  func_t& function = target;
  return function(value);
}
