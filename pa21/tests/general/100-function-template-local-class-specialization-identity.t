// VALIDATION: compile-pass
// A local class declared in a function template has a distinct identity in
// each function-template specialization.

template<class T>
void observe(T *)
{
}

template<class T>
void visit(T *first, T *last)
{
  struct cleanup
  {
    T *first;
    T *last;

    ~cleanup()
    {
      while (first != last)
      {
        observe(first);
        ++first;
      }
    }
  };

  cleanup value = {first, last};
}

int main()
{
  int integers[2] = {0, 0};
  long longs[2] = {0, 0};
  visit(integers, integers + 2);
  visit(longs, longs + 2);
  return 0;
}
