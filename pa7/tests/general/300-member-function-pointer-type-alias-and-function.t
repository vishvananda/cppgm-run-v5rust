struct C;
typedef int (C::* member_fn_ptr)() const;

member_fn_ptr f(member_fn_ptr p) {
  return p;
}
