namespace Q {
template<class T>
struct V {};
}

namespace N {
Q::V<Q::V<int> *> f();
}

int main()
{
  return 0;
}
