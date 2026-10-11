template<class C> class BitConstRef;
template<class T> struct HasStorage { static const bool value = false; };

template<int N, int M> class Bitset;

template<int N, int M>
struct HasStorage<Bitset<N, M> > { static const bool value = true; };

template<class C, bool = HasStorage<C>::value>
class BitRef {
  typedef typename C::storage_type storage_type;
  friend typename C::self;
  friend class BitConstRef<C>;
};

template<class C>
class BitConstRef {
  typedef typename C::storage_type storage_type;
};

template<class C, bool IsConst, typename C::storage_type = 0>
class BitIter {};

template<int N, int M>
class Bitset {
public:
  typedef int storage_type;
protected:
  typedef Bitset self;
  friend class BitRef<Bitset>;
  friend class BitConstRef<Bitset>;
  friend class BitIter<Bitset, false>;
  friend class BitIter<Bitset, true>;
public:
  typedef BitRef<Bitset> reference;
  typedef BitConstRef<Bitset> const_reference;
  typedef BitIter<Bitset, false> iterator;
  typedef BitIter<Bitset, true> const_iterator;
};

Bitset<0, 0>::iterator *p;
