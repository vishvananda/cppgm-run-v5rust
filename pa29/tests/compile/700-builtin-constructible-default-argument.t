// VALIDATION: compile-pass
// Hosted builtin-trait compatibility: __is_constructible must honor trailing
// constructor default arguments while trying all one-argument-callable ctors.

struct allocator
{
  allocator()
  {
  }
};

struct vector_like
{
  vector_like(const allocator &)
  {
  }

  vector_like(unsigned long, const allocator & = allocator())
  {
  }
};

static_assert(__is_constructible(vector_like, unsigned long),
              "defaulted allocator parameter should be viable");
static_assert(!__is_constructible(vector_like, int *),
              "unrelated argument is not viable");

int main()
{
  return 0;
}
