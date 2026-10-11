template<class Tag, class Op, class... Args>
inline const bool v = false;

struct Eq {
  template<class A, class B>
  bool operator()(const A&, const B&) const {
    return true;
  }
};

template<class A, class B>
inline const bool v<int, Eq, A, B> = true;

static_assert(v<int, Eq, int, int>, "");

int main() {
  return 0;
}
