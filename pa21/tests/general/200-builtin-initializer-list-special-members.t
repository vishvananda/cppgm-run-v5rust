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

int main() {
  std::initializer_list<char> il;
  return (int)il.size();
}
