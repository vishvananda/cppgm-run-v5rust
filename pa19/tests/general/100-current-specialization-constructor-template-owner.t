template<class T>
struct holder {
  T* ptr;

  template<class U>
  explicit holder(U* p) : ptr(p) {}

  template<class U>
  static holder make(U* p) {
    return holder(p);
  }
};

int main() {
  holder<int> h = holder<int>::make((int*)0);
  return h.ptr ? 1 : 0;
}
