// VALIDATION: compile-pass
// An explicit specialization matches the primary whose reconstructed concrete
// parameter type agrees, not another overload with the same dependent prefix.

template<class>
struct traits
{
  typedef unsigned carrier;
};

template<class T, class Traits>
int convert(typename Traits::carrier, T);

template<class T, class Traits>
long convert(typename Traits::carrier, T *);

template<>
long convert<int, traits<int> >(unsigned, int *)
{
  return 1;
}

long use(int * pointer)
{
  return convert<int, traits<int> >(1, pointer);
}
