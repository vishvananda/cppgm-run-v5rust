template<class T, T v>
struct integral_constant { static const T value = v; };

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<bool B, class T = void>
struct enable_if_c { typedef T type; };

template<class T>
struct enable_if_c<false, T> {};

template<class T>
struct remove_reference { typedef T type; };

template<class T>
struct remove_reference<T&> { typedef T type; };

template<class T>
struct remove_reference<T&&> { typedef T type; };

template<class Base, class Derived>
using is_base_of = false_type;

template<class... Cond>
struct all_false : false_type {};

template<class... T>
struct all_false<integral_constant<T, false>...> : true_type {};

template<class... T>
struct vector {
  vector() {}

  template<class... U,
           class = typename enable_if_c<
             all_false<
               is_base_of<vector, typename remove_reference<U>::type>...
             >::value
           >::type>
  explicit vector(U&&... u) {}
};

int main()
{
  vector<int, int, int, char const*> v(1, 2, 3, (char const*)0);
  return 0;
}
