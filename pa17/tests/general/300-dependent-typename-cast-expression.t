// VALIDATION: a C-style cast whose type-id is a typename-specifier naming a
// dependent member type is a cast, not a parenthesized expression.

template <class T>
struct promote
{
  typedef T type;
};

template <>
struct promote<float>
{
  typedef double type;
};

template <class T>
bool is_finite(T value)
{
  return __builtin_isfinite((typename promote<T>::type)value);
}

template <class T>
long widen(T value)
{
  return (typename promote<T>::type)value * 2;
}

int main()
{
  if (!is_finite(1.5f)) return 1;
  if (!is_finite(2.5)) return 2;
  if (widen(21) != 42) return 3;
  return 0;
}
