// VALIDATION: compile-pass
// N3485 focus: 12.3.2 [class.conv.fct], 14.8.2.3 [temp.deduct.conv]

template<class Tag>
struct result
{
  int value;

  explicit result(int input)
    : value(input)
  {
  }
};

struct source
{
  template<class Type>
  operator Type() const
  {
    return Type(7);
  }
};

template<class Result>
Result make_result()
{
  return source();
}

int main()
{
  result<int> first = make_result<result<int> >();
  const result<int> second = make_result<const result<int> >();
  return first.value == 7 && second.value == 7 ? 0 : 1;
}
