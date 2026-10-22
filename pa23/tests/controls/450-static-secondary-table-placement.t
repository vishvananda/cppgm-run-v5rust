// Itanium 2.5, N3485 3.6.2: static initialization installs each complete
// object's table address point, including secondary bases. Failed promotion
// still preserves the support objects it demanded. Expect exit 0.
struct Base {
  virtual int value() { return 1; }
  virtual ~Base() {}
};
struct Left : Base {};
struct Right : Base {};
struct Diamond : Left, Right {
  int value() { return 7; }
};
Diamond global;
int read(Base* p) { return p->value(); }
int constructions;
int next() { return ++constructions; }
struct Dynamic : Left, Right {
  int n;
  Dynamic() : n(next()) {}
  int value() { return n; }
};
int local_value()
{
  static Diamond local;
  static Dynamic dynamic;
  return read(static_cast<Right*>(&local)) + read(static_cast<Right*>(&dynamic));
}
int main()
{
  return read(static_cast<Right*>(&global)) == 7 &&
         dynamic_cast<Diamond*>(static_cast<Right*>(&global)) == &global &&
         local_value() == 8 && local_value() == 8 && constructions == 1 ? 0 : 1;
}
