// A declaration-only inherited source plan can precede the direct
// default-constructor query that defines its subobjects; expect exit 0
struct Member { int value; Member() : value(7) {} };
struct Base { Member member; Base() = default; };
struct Derived : Base { using Base::Base; Derived(int) {} };
int main() {
  Derived first;
  Base second;
  return first.member.value != 7 || second.member.value != 7;
}
