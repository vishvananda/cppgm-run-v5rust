// expect exit 0
// N3485 12.3.2, 3.4.5p7: aliases name canonical conversion result types;
// ordinary member calls keep their receiver and declared result categories.
namespace values {
struct Result { int value; };
Result stored = {7};
}
struct Source {
  typedef const values::Result& ResultAlias;
  explicit operator ResultAlias() const;
  explicit operator int() & { return 3; }
  explicit operator int() const & { return 5; }
  int through_this() { return operator int() + this->operator int(); }
};
Source::operator ResultAlias() const { return values::stored; }
struct Derived : Source {
  int qualified() { return Source::operator int(); }
};
struct Further : Derived {
  int through_naming_class() { return Derived::operator int(); }
};
struct Unused { explicit operator long(); };
int main() {
  typedef const values::Result& Result;
  Source source;
  const Source& constant = source;
  Derived derived;
  Further further;
  Result result = source.operator Result();
  return &result != &values::stored || source.through_this() != 6 ||
         constant.operator int() != 5 || derived.qualified() != 3 ||
         derived.Source::operator int() != 3 || further.through_naming_class() != 3;
}
