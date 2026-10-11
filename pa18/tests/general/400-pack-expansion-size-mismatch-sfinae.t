// VALIDATION: compile-pass
// A pack expansion that mentions multiple bound packs with different lengths is
// a substitution failure, not an unexpanded viable default argument.

template<bool B>
struct enable_if
{
};

template<>
struct enable_if<true>
{
  typedef void type;
};

template<class From, class To>
struct is_convertible
{
  static const bool value = true;
};

template<class... Pred>
struct and_;

template<>
struct and_<>
{
  static const bool value = true;
};

template<class First, class... Rest>
struct and_<First, Rest...>
{
  static const bool value = First::value && and_<Rest...>::value;
};

template<class T>
T &&declval();

template<class... T>
struct tuple_like
{
  tuple_like() {}

  template<class... U,
           class = typename enable_if<
             and_<is_convertible<U, T>...>::value && sizeof...(U) >= 1
           >::type>
  explicit tuple_like(U&&...) {}
};

struct yes
{
  char value;
};

struct no
{
  char value[2];
};

struct is_constructible
{
  template<class T, class... Args, class = decltype(T(declval<Args>()...))>
  static yes test(int);

  template<class, class...>
  static no test(...);
};

int main()
{
  return sizeof(is_constructible::test<tuple_like<>, int>(0)) == sizeof(no) &&
         sizeof(is_constructible::test<tuple_like<int>, int, int>(0)) == sizeof(no) &&
         sizeof(is_constructible::test<tuple_like<int, int>, int>(0)) == sizeof(no) &&
         sizeof(is_constructible::test<tuple_like<int, int>, int, int, int>(0)) == sizeof(no) ?
      0 :
      1;
}
