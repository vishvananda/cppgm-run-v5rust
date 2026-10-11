int take_unsigned(unsigned long) { return 0; }
int take_pointer(int *) { return 0; }
int storage[3] = {7};
int (&array_reference())[3] { return storage; }

template<class T> int fixed_calls()
{
  if (take_unsigned(sizeof(T) + 2) != 0) return 1;
  if (take_pointer(0) != 0 || take_pointer(nullptr) != 0) return 2;
  return array_reference()[0] != 7;
}

template<class T> int deferred_value()
{
  return 5 / sizeof(T);
}

template<class T> int dependent_operand(T value)
{
  return *value + sizeof(T);
}

template<class T> int shorted(int (&array)[sizeof(T) ? 3 : 1 / 0])
{
  return sizeof(array);
}

int main()
{
  int array[3] = {7};
  if (fixed_calls<int>() != 0 || fixed_calls<long>() != 0) return 1;
  if (dependent_operand(array) != 7 + sizeof(int *)) return 2;
  if (deferred_value<int>() != 5 / sizeof(int)) return 3;
  if (deferred_value<char>() != 5) return 4;
  return shorted<int>(array) != sizeof(array);
}
