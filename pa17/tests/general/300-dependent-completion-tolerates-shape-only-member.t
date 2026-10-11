// Naming `typename holder<T, D>::pointer` inside another template-id in a
// partial specialization's argument list completes holder<T, D> while T and D
// are still parameters.  That completion is a shape, not a layout: a member
// sized over the parameters folds to a meaningless bound because the stand-in
// has no size, and rejecting it there rejects the whole specialization.
//
// The tolerance must not leak into a real layout, so this also pins the size
// of a concrete instantiation, and it must not swallow `T[N]`, whose bound is
// a parameter and has a dependent-array representation of its own.

template<class T, class D>
struct holder
{
  typedef T *pointer;
  char pad_[sizeof(T) - 1];
};

template<class A, class B>
struct helper {};

template<class T>
struct hash;

template<class T, class D>
struct hash<helper<holder<T, D>, typename holder<T, D>::pointer> >
{
  static const int selected = 1;
};

template<class T>
struct bound_kind
{
  static const int value = 0;
};

template<class T, unsigned long N>
struct bound_kind<T[N]>
{
  static const int value = 1;
};

int main()
{
  int status = 0;
  // The concrete completion computes the real bound: sizeof(int) - 1 == 3.
  if (sizeof(holder<int, char>) != 3) status = 1;
  if (hash<helper<holder<int, char>, int *> >::selected != 1) status = 2;
  if (bound_kind<char[4]>::value != 1) status = 3;
  return status;
}
