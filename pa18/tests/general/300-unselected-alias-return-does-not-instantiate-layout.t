// VALIDATION: compile-pass
// Forming an unselected function-template candidate with an alias return type
// does not instantiate the returned class layout.

template<class T>
struct bomb
{
  static_assert(sizeof(T) == 0, "unselected return class instantiated");
};

template<class T>
using bomb_alias = bomb<T>;

template<class T>
bomb_alias<T> choose(T)
{
  return {};
}

int choose(int)
{
  return 0;
}

int use()
{
  return choose(1);
}
