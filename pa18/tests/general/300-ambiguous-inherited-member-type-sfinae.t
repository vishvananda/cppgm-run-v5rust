template<bool, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };

template<class, class> struct is_same { static const bool value = false; };
template<class T> struct is_same<T, T> { static const bool value = true; };

struct left { typedef left marker; };
struct right { typedef right marker; };
struct derived : left, right {};

template<class T, class = void> struct relocatable
{
  static const bool value = false;
};

template<class T>
struct relocatable<
    T,
    typename enable_if<is_same<T, typename T::marker>::value>::type>
{
  static const bool value = true;
};

static_assert(!relocatable<derived>::value,
              "ambiguous inherited member lookup must be a substitution failure");

int main()
{
  return relocatable<derived>::value;
}
