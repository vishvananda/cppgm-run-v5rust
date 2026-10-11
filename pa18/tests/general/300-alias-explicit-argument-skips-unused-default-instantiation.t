// VALIDATION: compile-pass
// N3485 focus: 14.7.1 [temp.inst]

template<class Type>
struct identity
{
  using type = Type;
};

template<class Type>
struct invalid_default
{
  using type = typename Type::missing;
};

template<class Type, class Default, class Nondefault = identity<Type>>
struct eval_default
{
  using type = typename Nondefault::type;
};

template<class Type, class Default, class Nondefault = identity<Type>>
using eval_default_t = typename eval_default<Type, Default, Nondefault>::type;

using selected = eval_default_t<int, invalid_default<int>>;

int main()
{
  return 0;
}
