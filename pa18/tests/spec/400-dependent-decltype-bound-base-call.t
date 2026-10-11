// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], dependent decltype operands.
// A decltype operand whose call target and argument mention still-dependent
// template-bound types remains dependent until the surrounding template is
// instantiated.

#include "../support.h"

template<class T>
struct identity
{};

struct yes
{};

struct no
{};

template<class Base, class T>
struct contains_result
{
  typedef decltype(Base::contains(identity<T>())) type;
};

struct concrete_base
{
  static yes contains(identity<int>);
};

template<class Base, class T>
struct is_contained
{
  static const bool value =
      is_same<typename contains_result<Base, T>::type, yes>::value;
};

int main()
{
  return is_contained<concrete_base, int>::value ? 0 : 1;
}
