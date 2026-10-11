// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.8.2 [temp.deduct]

template<class T> struct alloc {};
template<class A, class B> struct pair {};
template<class T, class A = alloc<T> > struct vec {};
template<class K, class V, class C = int, class A = alloc<pair<K, V> > > struct map {};

template<class T>
T * lookup(const map<int, vec<pair<int, T *> > > & box)
{
  return 0;
}

struct node {};

node * g(const map<int, vec<pair<int, node *> > > & box)
{
  return lookup(box);
}

int main()
{
  return 0;
}
