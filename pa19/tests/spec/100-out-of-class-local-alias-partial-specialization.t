// N3485 focus: 14.5.5 [temp.class.spec], 14.6 [temp.res]

struct Node {};

template<class T>
struct Alloc
{
  typedef T * pointer;

  void deallocate(pointer, unsigned long) {}
};

template<class AllocT, class Obj>
struct Rebind
{
  typedef Alloc<Obj> type;
};

template<class P>
struct PointerTraits
{
  enum { selected = 0 };
};

template<class T>
struct PointerTraits<T *>
{
  typedef T * pointer;
  enum { selected = 1 };

  static pointer pointer_to(T & value)
  {
    return &value;
  }
};

template<class T, class A>
struct Control
{
  int release_weak();
};

template<class T, class A>
int Control<T, A>::release_weak()
{
  typedef typename Rebind<A, Control>::type Al;
  typedef PointerTraits<typename Al::pointer> PTraits;

  Al alloc;
  alloc.deallocate(PTraits::pointer_to(*this), 1);
  return PTraits::selected;
}

int main()
{
  Control<Node, Alloc<Node> > control;
  return control.release_weak() == 1 ? 0 : 1;
}
