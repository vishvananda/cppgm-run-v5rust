// VALIDATION: compile-pass
// N3485 focus: 15.1 [except.throw]
// Initializing the exception object ODR-uses the selected constructor. An
// inline constructor of a class-template specialization must be instantiated
// so its definition is available to object lowering.

template<class T>
struct thrown_value
{
  explicit thrown_value(T initial)
    : value(initial)
  {
  }

  thrown_value(thrown_value && other)
    : value(other.value)
  {
    other.value = 0;
  }

  T value;
};

int main()
{
  thrown_value<int> source(7);
  try {
    throw static_cast<thrown_value<int> &&>(source);
  } catch(thrown_value<int> & caught) {
    return caught.value == 7 && source.value == 0 ? 0 : 1;
  }
}
