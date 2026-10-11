template<class T, class U>
struct Pick {
  typedef int type;
  int value;
};

template<class T>
struct Pick<T, T*> {
  typedef long type;
  long value;
};

template<class T>
struct RefUse {
  typedef typename Pick<T, T*>::type type;
};

static_assert(sizeof(RefUse<int>::type) == sizeof(long), "");

Pick<int, int*> make() {
  Pick<int, int*> p;
  p.value = 7;
  return p;
}

int main() {
  return make().value;
}
