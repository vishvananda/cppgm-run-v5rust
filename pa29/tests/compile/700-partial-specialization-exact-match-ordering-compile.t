struct Less {};

template<class K, class V, class C>
struct MapValueCompare {};

template<class C, class L, class R, class = void>
struct Lazy {
  Lazy(const C&) {}
  int which() const { return 0; }
};

template<class V, class K, class C>
struct Lazy<MapValueCompare<K, V, C>, V, V> {
  Lazy(const MapValueCompare<K, V, C>&) {}
  int which() const { return 1; }
};

template<class V, class K, class TK, class C>
struct Lazy<MapValueCompare<K, V, C>, TK, V> {
  Lazy(const MapValueCompare<K, V, C>&) {}
  int which() const { return 2; }
};

struct K {};
struct V {};

int main() {
  MapValueCompare<K, V, Less> cmp;
  Lazy<MapValueCompare<K, V, Less>, V, V> x(cmp);
  return x.which() == 1 ? 0 : 1;
}
