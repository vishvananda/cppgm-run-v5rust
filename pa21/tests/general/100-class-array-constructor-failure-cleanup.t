int live;
int freed;
struct X {
  X() { if(live == 2) throw 1; ++live; }
  ~X() { --live; }
  static void *operator new[](decltype(sizeof(0)) n) { return ::operator new[](n); }
  static void operator delete[](void *p) noexcept { ++freed; ::operator delete[](p); }
};
int main() {
  try { new X[3]; } catch(...) { return live || freed != 1; }
  return 1;
}
