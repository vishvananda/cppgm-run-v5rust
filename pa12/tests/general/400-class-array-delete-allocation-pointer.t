void *allocated;
int good;
struct X {
  X() {} ~X() {}
  static void *operator new[](decltype(sizeof(0)) n) { return allocated = ::operator new[](n); }
  static void operator delete[](void *p) noexcept { good = p == allocated; ::operator delete[](p); }
};
int main() { delete[] new X[2]; return !good; }
