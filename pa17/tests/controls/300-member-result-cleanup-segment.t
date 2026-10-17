// expect exit 0
// A nonthrowing member-class-template result retains outer cleanup demand;
// a root result initializing an object is distinct from operand temporaries.
int destructions;
struct Outer
{
  ~Outer() noexcept { ++destructions; }
};
struct Owner
{
  template<class T> struct Result
  {
    Result() noexcept {}
    ~Result() noexcept { ++destructions; }
    int value() const noexcept { return 7; }
  };
};
Owner::Result<int> make() noexcept { return Owner::Result<int>(); }
void consume(const Owner::Result<int>& value) noexcept
{
  if (value.value() != 7) destructions += 100;
}
struct Use
{
  Use(int value) noexcept { if (value != 7) destructions += 100; }
};
void exercise()
{
  Outer outer;
  make();
  consume(make());
  bool choose = destructions != 42;
  (choose ? make() : make()).value();
  Use nested(make().value());
  Use braced{make().value()};
  Owner::Result<int> result = make();
}
int main()
{
  exercise();
  return destructions != 7;
}
