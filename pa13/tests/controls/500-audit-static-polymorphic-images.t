// Static polymorphic images retain the dynamic type, converted members and
// nested object tables; expect exit 0
struct Empty { virtual int value() const { return 3; } };
Empty empty;
Empty empty_copy = empty;
Empty empty_array[2]{};
struct Base { virtual ~Base() {} virtual int value() const { return 5; } };
struct Derived : Base { int value() const override { return 7; } };
Derived derived;
int target = 11;
struct Promoted {
  unsigned char byte;
  int *pointer;
  int *null;
  Promoted(int n = 258) : byte(n), pointer(&target), null(nullptr) {}
  virtual int value() const { return byte + *pointer + (null == nullptr); }
};
Promoted promoted;
struct Owner { Base member; virtual ~Owner() {} };
Owner owner;
struct Zero { int values[2]; virtual int value() const { return values[0] + values[1]; } };
Zero zero{};
struct Bits { unsigned a:3; unsigned b:7; char c; virtual int value() const { return a+b+c; } };
Bits bits{};
union Union { int n; };
struct UnionOwner { Union u; virtual int value() const { return u.n; } };
UnionOwner union_owner{};
int main() {
  const Base &reference = derived;
  return empty.value() == 3 && reference.value() == 7 && promoted.value() == 14
      && owner.member.value() == 5 && zero.value() == 0 && empty_copy.value() == 3
      && empty_array[1].value() == 3 && bits.value() == 0 && union_owner.value() == 0 ? 0 : 1;
}
