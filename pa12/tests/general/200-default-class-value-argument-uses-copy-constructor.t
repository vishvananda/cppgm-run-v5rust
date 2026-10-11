// VALIDATION: compile-pass
// Materializing a class-valued default argument for a by-value constructor
// parameter invokes the class's user-provided copy constructor.

struct copied_value
{
  int value;

  copied_value()
    : value(4)
  {
  }

  copied_value(const copied_value & other)
    : value(other.value + 1)
  {
  }
};

copied_value default_source;

struct consumer
{
  int observed;

  consumer(copied_value input = default_source)
    : observed(input.value)
  {
  }
};

int main()
{
  consumer value;
  return value.observed == 5 ? 0 : 1;
}
