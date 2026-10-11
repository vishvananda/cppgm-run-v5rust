template<class T, class P, class R, class M, long B>
struct X
{
  X() {}

  template<class Pp, class Rp, class Mp, int = 0>
  X(const X<T, Pp, Rp, Mp, B> &)
  {
  }
};

int main()
{
  X<int, int *, int &, int **, 1024> it;
  X<int, int const *, int const &, int const * const *, 1024> cit(it);
  (void)cit;
  return 0;
}
