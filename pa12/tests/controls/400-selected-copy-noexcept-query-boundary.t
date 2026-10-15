// Frozen conversion_may_throw queries a selected implicit copy's callable;
// written construction separately queries its trailing default arguments.
// expect exit 0
int argument();
struct Base {
  Base() noexcept;
  Base(const Base&, int = argument()) noexcept;
  ~Base() noexcept;
};
struct Derived : Base { Derived() noexcept; };
extern Derived source;
void take(Base) noexcept;
int main() {
  if (!noexcept(take(source))) return 1;
  if (!noexcept(take(Derived()))) return 2;
  if (noexcept(Base(source))) return 3;
  if (noexcept(Base(Derived()))) return 4;
  return 0;
}
