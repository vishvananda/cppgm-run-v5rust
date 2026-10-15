// expect exit 0
// N3485 7.3.3p15-p17: imports keep the base receiver and take the using
// declaration's access; an own declaration hides an imported signature.
struct Base {
protected:
  operator int() { return 1; }
  operator int() const { return 2; }
};
struct Imported : Base {
  typedef int Alias;
  using Base::operator Alias;
  operator long() { return 4; }
};
struct DeclaredAfter : Base {
  using Base::operator int;
  operator int() { return 3; }
};
struct DeclaredBefore : Base {
  operator int() { return 5; }
  using Base::operator int;
};
int main() {
  Imported imported;
  const Imported& constant = imported;
  DeclaredAfter after;
  DeclaredBefore before;
  return imported.operator int() != 1 || constant.operator int() != 2 ||
         int(imported) != 1 || after.operator int() != 3 ||
         before.operator int() != 5;
}
