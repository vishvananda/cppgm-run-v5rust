// Each unit expands these bodies (always_inline), so a function's static
// variable is one object across the units only when both name it alike.
__attribute__((always_inline)) inline int counter()
{
  static int value = 0;
  return ++value;
}

struct Box
{
  __attribute__((always_inline)) int bump() const
  {
    static int calls = 0;
    return ++calls;
  }
};

__attribute__((always_inline)) inline int local_class_counter()
{
  struct Local
  {
    __attribute__((always_inline)) static int next()
    {
      static int n = 0;
      return ++n;
    }
  };
  return Local::next();
}
