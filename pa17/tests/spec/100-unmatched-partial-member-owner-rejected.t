// VALIDATION: compile-fail
// N3485 focus: 14.5.5.3 [temp.class.spec.mfunc]
// An out-of-class member definition must name the primary pattern or an
// existing partial specialization exactly. It cannot create a new owner
// pattern by changing T to T *.

template<class T>
struct owner
{
  int value();
};

template<class T>
int owner<T *>::value()
{
  return 1;
}
