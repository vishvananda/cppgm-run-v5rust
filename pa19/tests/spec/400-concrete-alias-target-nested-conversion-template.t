// VALIDATION: compile-pass
// N3485 focus: 7.1.3 [dcl.typedef], 14.5.7 [temp.alias],
// 14.8.2.3 [temp.deduct.conv]
// A concrete alias-template use retains its concrete class specialization,
// including a nested conversion function template from another specialization.

namespace leaf
{
  template<class T>
  struct result
  {
    explicit result(int input)
      : value(input)
    {
    }

    struct error_result
    {
      int value;

      template<class U>
      operator result<U>() const
      {
        return result<U>(value);
      }
    };

    error_result error() const
    {
      return error_result{value};
    }

    int value;
  };
}

template<class T>
using result = leaf::result<T>;

result<int> convert_error()
{
  result<char> source(9);
  return source.error();
}

int main()
{
  result<int> value = convert_error();
  return value.value == 9 ? 0 : 1;
}
