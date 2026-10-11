template<class Tag, class Op, class... Args>
inline const bool v = false;

template<class Tag, class Op, class... Args>
inline const bool v<Tag, Op volatile, Args...> = v<Tag, Op, Args...>;

template<class Tag, class Op, class... Args>
inline const bool v<Tag, Op const volatile, Args...> = v<Tag, Op volatile, Args...>;

struct Eq {
  template<class A, class B>
  bool operator()(const A&, const B&) const {
    return true;
  }
};

template<class A, class B>
inline const bool v<long, Eq, A, B> = true;

static_assert(v<long, Eq volatile, int, int>, "");
static_assert(v<long, Eq const volatile, int, int>, "");

int main() {
  return 0;
}
