// N3485 12.1p5: a default constructor of a class with virtual functions is
// not trivial, even when explicitly defaulted; it installs the vpointer of
// its class after its base constructors have run.
struct Base
{
  Base() = default;
  virtual ~Base() {}
  virtual int id() const { return 1; }
};

struct Derived : Base
{
  Derived() = default;
  int id() const override { return 2; }
};

struct Further : Derived
{
  Further() noexcept = default;
  int id() const override { return 3; }
};

int read(const Base& base) { return base.id(); }

int main()
{
  Derived local;
  Further further;
  Base* made = new Derived;
  Base* value = new Further();
  int result = read(local) * 1000 + read(further) * 100 + made->id() * 10 +
    value->id();
  delete made;
  delete value;
  return result == 2323 ? 0 : 1;
}
