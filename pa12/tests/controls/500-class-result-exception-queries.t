// expect exit 0
struct Safe {
  ~Safe() noexcept;
};
struct Throwing {
  ~Throwing() noexcept(false);
};
Safe safe() noexcept;
Safe may_throw();
Throwing throwing() noexcept;
Safe (*indirect)() noexcept;

int main()
{
  return !noexcept(safe()) || noexcept(may_throw()) || noexcept(throwing()) ||
         !noexcept(indirect()) || !noexcept(true ? safe() : safe()) ||
         !noexcept((0, safe()));
}
