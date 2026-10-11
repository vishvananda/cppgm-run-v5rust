// VALIDATION: function templates with the same template-head and parameter
// list but different decltype results are distinct templates; a default
// template argument on each is not a duplicate default.

template <bool Condition, class T = void>
struct enable_if
{
};

template <class T>
struct enable_if<true, T>
{
  typedef T type;
};

template <class Left, class Right>
struct is_same
{
  static const bool value = false;
};

template <class T>
struct is_same<T, T>
{
  static const bool value = true;
};

template <class T>
T&& declval();

struct first_kind
{
  int value() { return 21; }
};

struct second_kind
{
  long count;
};

template <class T, class = typename enable_if<is_same<T, first_kind>::value>::type>
decltype(declval<T>().value())
pick(T& object)
{
  return object.value();
}

template <class T, class = typename enable_if<is_same<T, second_kind>::value>::type>
decltype(declval<T>().count)
pick(T& object)
{
  return object.count;
}

int main()
{
  first_kind first;
  second_kind second = {42};
  if (pick(first) != 21) return 1;
  if (pick(second) != 42) return 2;
  return 0;
}
