// VALIDATION: compile-pass
// N3485 focus: core-language reduction of iterator_traits alias surface

template<typename Iterator>
struct simple_iterator_traits;

template<typename T>
struct simple_iterator_traits<T *>
{
  typedef T value_type;
};

template<typename Iterator>
struct wrapper
{
  typedef typename simple_iterator_traits<Iterator>::value_type value_type;
};

int main()
{
  wrapper<int *> value;
  (void)sizeof(typename wrapper<int *>::value_type);
  (void)value;
  return 0;
}
