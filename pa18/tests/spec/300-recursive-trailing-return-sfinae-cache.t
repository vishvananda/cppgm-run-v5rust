// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], expression SFINAE in a dependent decltype.

int probe(...);

template<class T>
auto probe(T value) -> decltype(probe(value))
{
  return probe(value);
}

int main()
{
  typedef decltype(probe(0)) result;
  result value = 0;
  return value;
}
