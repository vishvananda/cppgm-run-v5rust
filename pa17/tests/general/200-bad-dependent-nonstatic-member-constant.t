// VALIDATION: compile-fail
// Substitution cannot turn a non-static data member into a constant expression;
// the member still requires an object.

struct owner
{
  const int value = 1;
};

template<class T>
struct use
{
  static const int result = T::value;
};

int invalid[use<owner>::result];
