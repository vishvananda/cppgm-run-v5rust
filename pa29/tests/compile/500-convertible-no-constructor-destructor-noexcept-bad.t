// expect reject
// Failed constructor selection returns null inside the soft conversion query;
// its existing destructor's exception specification is still demanded.
template<class T> struct A {
  A(int);
  ~A() noexcept(sizeof(typename T::missing)>0);
};
static_assert(!__is_convertible(int*,A<int>), "");
