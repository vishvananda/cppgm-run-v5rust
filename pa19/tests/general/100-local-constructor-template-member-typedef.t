template<class T>
struct IteratorTraits;

template<class T>
struct IteratorTraits<T *>
{
  typedef T & reference;
};

template<class Iter>
struct Wrap
{
  typedef typename IteratorTraits<Iter>::reference reference;
  Iter p;

  Wrap() : p(0) {}

  template<class OtherIter>
  Wrap(const Wrap<OtherIter> & other) : p(other.p) {}
};

template<class T>
struct MiniVec
{
  typedef Wrap<T *> iterator;
  typedef Wrap<const T *> const_iterator;
};

int main()
{
  struct ActiveInterval
  {
    int x;
  };

  MiniVec<ActiveInterval>::iterator it;
  MiniVec<ActiveInterval>::const_iterator cit(it);
  return 0;
}
