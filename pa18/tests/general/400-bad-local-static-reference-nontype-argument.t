// VALIDATION: compile-fail
// A function-local static object has no linkage and is not a valid C++11
// reference non-type template argument.

template<int &Reference>
struct reference_holder
{
  static int read()
  {
    return Reference;
  }
};

int main()
{
  static int local_value = 4;
  return reference_holder<local_value>::read();
}
