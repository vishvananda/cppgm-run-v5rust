// A function type used as a builtin constructibility argument is a function
// designator lvalue and can bind directly to a function lvalue reference.
using function_type = void(int);

struct holder
{
  holder(function_type &);
};

static_assert(__is_constructible(holder, function_type),
              "function type should bind as a function designator lvalue");

int main()
{
  return 0;
}
