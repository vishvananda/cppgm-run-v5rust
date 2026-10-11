// VALIDATION: compile-pass
// An explicit type template argument may be a function type.

template<class T>
int accepts(T *pointer)
{
  return pointer ? 0 : 1;
}

void target(int)
{
}

int main()
{
  return accepts<void(int)>(&target);
}
