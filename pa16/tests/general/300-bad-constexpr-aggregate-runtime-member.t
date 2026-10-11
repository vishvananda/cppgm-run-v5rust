// VALIDATION: compile-fail
// Every member of a constexpr aggregate object must have a constant
// initializer.

int runtime_value();

struct pair
{
  int first;
  int second;
};

constexpr pair value = {runtime_value(), 2};

int main()
{
  return value.second;
}
