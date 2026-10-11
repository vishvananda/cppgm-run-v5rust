// VALIDATION: compile-fail
// A function-local static object has no linkage and its address is not a valid
// C++11 pointer non-type template argument.

template<int * Pointer>
struct pointer_holder
{
  static int read()
  {
    return *Pointer;
  }
};

int main()
{
  static int local_value = 4;
  return pointer_holder<&local_value>::read();
}
