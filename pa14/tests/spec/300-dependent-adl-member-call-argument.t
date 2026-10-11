// VALIDATION: compile-pass
// A class-template member body may call an unqualified function with arguments
// formed from member calls on a dependent base alias. Ordinary lookup is empty
// at the definition point, but ADL can find the function when the iterator type
// is known at instantiation.

namespace user {

struct iter
{
  int value;
};

struct list_like
{
  iter begin()
  {
    iter result;
    result.value = 7;
    return result;
  }

  iter end()
  {
    iter result;
    result.value = 9;
    return result;
  }
};

template<class T>
struct base
{
  list_like values;
};

iter find(iter first, iter, int)
{
  return first;
}

}  // namespace user

template<class T>
struct check : user::base<T>
{
  typedef user::base<T> super_type;

  int run(int needle)
  {
    user::iter it = find(super_type::values.begin(),
                         super_type::values.end(),
                         needle);
    return it.value;
  }
};

int main()
{
  check<int> c;
  return c.run(3) == 7 ? 0 : 1;
}
