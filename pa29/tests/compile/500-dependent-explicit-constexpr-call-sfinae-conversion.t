template<bool Condition, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<bool Condition, class T = void>
using enable_if_t = typename enable_if<Condition, T>::type;

struct iterator {};

struct const_iterator
{
  const_iterator(iterator);
};

template<class First, class Second>
struct pair
{
  struct _CheckArgs
  {
    template<class _U1, class _U2>
    static constexpr bool __enable_implicit()
    {
      return __is_constructible(First, _U1) &&
             __is_constructible(Second, _U2) &&
             __is_convertible(_U1, First) &&
             __is_convertible(_U2, Second);
    }
  };

  template<class _U1, class _U2,
           enable_if_t<
               _CheckArgs::template __enable_implicit<const _U1&, const _U2&>(),
               int> = 0>
  pair(const pair<_U1, _U2>&);
};

pair<const_iterator, bool> convert(pair<iterator, bool> value)
{
  return value;
}

int main() { return 0; }
