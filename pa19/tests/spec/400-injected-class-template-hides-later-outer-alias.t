// VALIDATION: compile-pass
// N3485 focus: 9 [class], 14.6.1 [temp.local], 14.5.7 [temp.alias]
// The injected class-template name in a deferred member-template declaration
// hides an outer alias template collected later in the translation unit.

namespace leaf
{
  template<class T>
  struct result
  {
    int value;

    template<class U>
    int move_from(result<U> &&)
    {
      return 7;
    }

    result()
      : value(0)
    {
    }

    result(result && other)
      : value(move_from(static_cast<result &&>(other)))
    {
    }
  };
}

template<class T>
using result = leaf::result<T>;

int main()
{
  leaf::result<int> source;
  leaf::result<int> moved(static_cast<leaf::result<int> &&>(source));
  return moved.value == 7 ? 0 : 1;
}
