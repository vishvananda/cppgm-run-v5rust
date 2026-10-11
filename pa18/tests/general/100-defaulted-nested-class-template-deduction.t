template<class T> struct Alloc {};
template<class A, class B> struct Pair {};
template<class T, class A = Alloc<T> > struct Vec {};
template<class K, class V, class C = int, class A = Alloc<Pair<K, V> > > struct Map {};

template<typename T>
T * lookup(const Map<int, Vec<Pair<int, T *> > > & box)
{
  return 0;
}

struct F {};

F * g(const Map<int, Vec<Pair<int, F *> > > & box)
{
  return lookup(box);
}

int main()
{
  return 0;
}
