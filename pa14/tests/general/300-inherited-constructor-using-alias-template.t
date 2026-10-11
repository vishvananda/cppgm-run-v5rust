struct YTraits {};

template<class T>
struct YBase {
  explicit YBase(const T&) {}
};

template<class T>
struct YDerived : public YBase<T> {
  typedef YBase<T> YAlias;
  using YAlias::YAlias;
};

// N3485 12.9: the terminal T names a constructor, not a new member.
template<class T>
struct DirectDerived : T {
  using T::T;
};

struct IntBase {
  int value;
  explicit IntBase(int n) : value(n) {}
};

template<class T>
struct YHolder {
  typedef YDerived<T> YMember;

  explicit YHolder(const T& t) : member(t) {}

  YMember member;
};

int main() {
  YTraits t;
  YHolder<YTraits> h(t);
  DirectDerived<IntBase> direct(7);
  return direct.value != 7;
}
