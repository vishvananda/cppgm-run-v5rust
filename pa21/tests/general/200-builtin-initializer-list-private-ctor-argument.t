namespace std {
template<class E>
class initializer_list {
  const E* __begin_;
  unsigned long __size_;

  initializer_list(const E* __b, unsigned long __s)
      : __begin_(__b), __size_(__s) {}

public:
  initializer_list() : __begin_(0), __size_(0) {}

  unsigned long size() const {
    return __size_;
  }
};
}

struct Item { Item(int) noexcept {} };
struct Box {
  unsigned long count;
  Box(std::initializer_list<Item> values) noexcept : count(values.size()) {}
};
int consume(const Box& box) noexcept { return (int)box.count; }
static_assert(noexcept(consume({1, 2})), "list argument is nonthrowing");

int may_throw();
static_assert(!noexcept(consume({may_throw(), 2})),
              "list element evaluation may throw");

int take(std::initializer_list<int> values) {
  return (int)values.size();
}

int main() {
  return take({1, 2, 3}) == 3 && consume({1, 2}) == 2 ? 0 : 1;
}
