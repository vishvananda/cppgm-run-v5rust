// VALIDATION: compile-pass

struct enabled_tag {};

template<class Key, class Value>
struct Pair {};

template<class T, class Tag>
struct MapBase {
  int get();
};

template<class Key, class Value>
struct MapBase<Pair<Key, Value>, enabled_tag> {
  typedef MapBase<Pair<Key, Value>, enabled_tag> __hashtable;

  int get();
};

template<class T, class Tag>
int MapBase<T, Tag>::get() {
  return 1;
}

template<class Key, class Value>
int MapBase<Pair<Key, Value>, enabled_tag>::get() {
  __hashtable *h = static_cast<__hashtable *>(this);
  return h ? 7 : 2;
}

int main() {
  MapBase<Pair<int, char>, enabled_tag> map;
  return map.get() == 7 ? 0 : 1;
}
