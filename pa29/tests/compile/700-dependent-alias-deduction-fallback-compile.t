template<class T, bool>
struct dependent_type : T {};

template<class D>
struct sfinae {
  typedef const D & lval_ref_type;
  typedef D && good_rval_ref_type;
};

struct Del {};

template<class T, class D>
struct U {
  typedef T * pointer;
  typedef sfinae<D> S;

  template<bool Dummy>
  using Bad = typename dependent_type<S, Dummy>::bad_rval_ref_type;

  template<bool Dummy = true>
  static int probe(pointer, Bad<Dummy>);

  static long probe(...);
};

int main()
{
  return sizeof(U<int *, Del>::probe((int **)0, Del())) == sizeof(long) ? 0 : 1;
}
