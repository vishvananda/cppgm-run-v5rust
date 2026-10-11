// VALIDATION: compile-pass
// N3485 focus: 14.5.5.3 [temp.class.spec.mfunc]

struct enabled_tag {};

template<class Key, class Value>
struct Pair {};

template<class T, class Tag>
struct MapBase
{
  int get();
};

template<class Key, class Value>
struct MapBase<Pair<Key, Value>, enabled_tag>
{
  int get();
};

template<class T, class Tag>
int MapBase<T, Tag>::get()
{
  return 1;
}

template<class Key, class Value>
int MapBase<Pair<Key, Value>, enabled_tag>::get()
{
  return 7;
}

int main()
{
  MapBase<Pair<int, char>, enabled_tag> map;
  return map.get() == 7 ? 0 : 1;
}
