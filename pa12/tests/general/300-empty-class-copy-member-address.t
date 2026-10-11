// An empty class subobject copied from a reference is still a class object.
// Lowering must use its address and must not invent a scalar payload load.
struct Empty
{
};

struct Wrapper
{
  Empty value;

  Wrapper(const Empty & input) : value(input)
  {
  }
};

int main()
{
  Empty input;
  Wrapper wrapper(input);
  (void)wrapper;
  return 0;
}
