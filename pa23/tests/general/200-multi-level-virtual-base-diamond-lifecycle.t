int live = 0;
struct V {
  V() noexcept { ++live; }
  ~V() noexcept { --live; }
  virtual void anchor() noexcept {}
};
struct A : virtual V {};
struct B : virtual V {};
struct AB : A, B {};
struct D : AB {};
int main() { { D value; if(live != 1) return 1; }
  return live == 0 ? 0 : 2; }
