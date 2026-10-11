// A non-type template argument whose value reads a variable template
// specialization that is still dependent waits on the enclosing template's
// arguments; it is not a non-constant expression.  The dependence also has to
// survive an operator, because the argument is normally written as a
// conjunction over several of the enclosing template's parameters.

template<bool B, class X, class Y>
struct conditional
{
  typedef X type;
};

template<class X, class Y>
struct conditional<false, X, Y>
{
  typedef Y type;
};

template<class T>
struct is_small
{
  static const bool value = sizeof(T) <= 4;
};

template<class T>
const bool small_v = is_small<T>::value;

template<class T, class U>
struct pair_kind
{
  typedef T first;
  typedef U second;

  // Both operands name a variable template specialization whose own argument
  // is a member typedef of this template, so neither has a value yet.
  typedef typename conditional<small_v<first> && small_v<second>,
    char, double>::type type;
};

int main()
{
  pair_kind<char, char>::type both_small = 0;
  pair_kind<char, double>::type one_large = 0;
  return (sizeof(both_small) == 1 && sizeof(one_large) == sizeof(double)) ?
    0 : 1;
}
