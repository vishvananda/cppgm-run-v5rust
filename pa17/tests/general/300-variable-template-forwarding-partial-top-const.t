template<class Tag, class Op, class... Args>
inline const bool v = false;

template<class Tag, class Op, class... Args>
inline const bool v<Tag, Op const, Args...> = v<Tag, Op, Args...>;

struct Eq {
  template<class A, class B>
  bool operator()(const A&, const B&) const {
    return true;
  }
};

template<class A, class B>
inline const bool v<int, Eq, A, B> = true;

static_assert(v<int, Eq const, int, int>, "");

int main() {
  return 0;
}
