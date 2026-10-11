struct C { int x; };
typedef int C::* member_ptr;

member_ptr f(member_ptr p) {
  return p;
}
