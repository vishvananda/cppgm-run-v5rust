// N3485 focus: 14.8.2 [temp.deduct] a qualified overload set is resolved
// after another call argument deduces the function-pointer parameter type.

namespace api
{
struct path
{
  int value;
};

bool create(path const& value)
{
  return value.value == 7;
}

bool create(path const&, int)
{
  return false;
}
}

template<class Argument>
int bind(bool (*function)(Argument const&), Argument const& argument)
{
  return function(argument) ? 0 : 1;
}

int main()
{
  api::path value = { 7 };
  return bind(api::create, value);
}
