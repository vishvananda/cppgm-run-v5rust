template <class K, class V>
struct Pair {
  K first;
  V second;
};

template <class K, class V>
struct OpaqueValue;

template <class T>
struct GetNodeValueType {
  typedef T type;
};

template <class K, class V>
struct GetNodeValueType<OpaqueValue<K, V> > {
  typedef Pair<const K, V> type;
};

template <class T>
using GetNodeValueTypeT = typename GetNodeValueType<T>::type;

namespace {

struct Payload {
  int value;
};

GetNodeValueTypeT<OpaqueValue<int, Payload> >* g();

int f() {
  return g()->second.value;
}

}

int main() {
  return 0;
}
