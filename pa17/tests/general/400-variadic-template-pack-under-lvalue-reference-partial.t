// VALIDATION: compile-pass
// A pack in a template-id remains deducible when the template-id is wrapped in
// an lvalue reference in a class partial-specialization argument.

template<class... T>
struct types
{
};

struct first
{
};

struct second
{
};

template<class T>
struct selected
{
  static const int value = 0;
};

template<class... T>
struct selected<types<T...> &>
{
  static const int value = sizeof...(T);
};

static_assert(selected<types<first, second> &>::value == 2, "");

int main()
{
  return selected<types<first, second> &>::value == 2 ? 0 : 1;
}
