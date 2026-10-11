// A variable template specialization initialized with a lambda
// (`_Z7next_idIiE`) and a static data member template defined out of its
// class (`_ZN5table5countIiEE`) are one object per specialization, shared
// with another unit only when both name it alike.  The closure itself is
// `_ZNK7next_idIiEMUlvE_clEv` as clang names it; GCC omits the `M`.
template<class T> int (*next_id)() = [] { static int id = 0; return ++id; };

struct table
{
  template<class T> static int count;
};

template<class T> int table::count = 10;
