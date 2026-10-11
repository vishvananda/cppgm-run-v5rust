static_assert(__is_pointer(int *), "pointer");
static_assert(__is_pointer(const int *), "pointer to const");
static_assert(!__is_pointer(int), "non-pointer");

static_assert(__is_reference(int &), "lvalue reference");
static_assert(__is_reference(const int &), "const lvalue reference");
static_assert(__is_reference(int &&), "rvalue reference");
static_assert(!__is_reference(int), "non-reference");

int main()
{
  return 0;
}
