// VALIDATION: compile-fail
// An out-of-class partial specialization may not skip a missing intermediate
// owner and fall back to an enclosing same-named member template.

template<class T>
struct outer
{
  template<class First, class Second>
  struct inner;
};

template<class T>
template<class U>
struct outer<T>::missing::inner<U, U *>
{
};

int main()
{
  return 0;
}
