// expect exit 0
struct Outer {
  ~Outer() noexcept(sizeof(Outer) > 0);
  struct Inner {
    ~Inner() noexcept(sizeof(Outer) > 0);
    struct Leaf {
      ~Leaf() noexcept(sizeof(Outer) > 0);
    } leaf;
  } inner;
  int value;
};

int main()
{
  return sizeof(Outer) == 8 ? 0 : 1;
}
