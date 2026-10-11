// N3485 focus: 14.7.3 [temp.expl.spec], 14.8.2 [temp.deduct]
// An explicit specialization may omit its template-argument-list when the
// primary's template argument is deduced from the complete function type,
// including its return type.

template<class T>
struct result
{
  int value;
};

template<class T>
result<T> make_result(int)
{
  return {1};
}

template<>
result<long> make_result(int)
{
  return {22};
}

int main()
{
  return make_result<long>(0).value == 22 ? 0 : 1;
}
