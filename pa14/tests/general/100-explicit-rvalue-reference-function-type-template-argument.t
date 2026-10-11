// VALIDATION: compile-pass
// An explicit function-type template argument retains an rvalue-reference
// parameter declarator.

template<class T>
int accepts(T *pointer)
{
  return pointer ? 0 : 1;
}

void target(int&&)
{
}

int main()
{
  return accepts<void(int&&)>(&target);
}
