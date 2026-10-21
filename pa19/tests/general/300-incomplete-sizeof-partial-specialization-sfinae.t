// Course rule (pa19/README.md): include CWG1315 beyond literal N3485 14.5.5p8.
// The dependent non-type partial argument is permitted because T is deducible
// from the first argument. sizeof(comparator<T>) on the incomplete class fails
// substitution, removing that specialization without rejecting this program.
// Required outcome: accept; lazy<X>::value() uses the fallback and returns zero.
// https://cplusplus.github.io/CWG/issues/1315.html

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

struct false_type
{
  static const bool value = false;
};

struct true_type
{
  static const bool value = true;
};

template<class T, class = void>
struct comparator;

template<class T, bool = true>
struct has_comparator : false_type {};

template<class T>
struct has_comparator<T, sizeof(comparator<T>) >= 0> : true_type {};

template<class T, class = void>
struct lazy
{
  static int value()
  {
    return 0;
  }
};

template<class T>
struct lazy<T, enable_if_t<has_comparator<T>::value> >
{
  static int value()
  {
    return sizeof(comparator<T>);
  }
};

struct X {};

int main()
{
  return lazy<X>::value();
}
