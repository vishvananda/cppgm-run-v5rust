namespace std {
template<class E>
class initializer_list {
  const E* __begin_;
  unsigned long __size_;

  initializer_list(const E* __b, unsigned long __s)
      : __begin_(__b), __size_(__s) {}

public:
  initializer_list() : __begin_(0), __size_(0) {}
};
}

struct Pick {
  int value;

  Pick(int, int) : value(99) {}
  Pick(std::initializer_list<int>) : value(2) {}
};

int main() {
  Pick p{4, 5};
  return p.value == 2 ? 0 : 1;
}
