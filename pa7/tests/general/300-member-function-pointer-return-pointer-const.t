struct D;
struct C {
  D const * get() const;
};

typedef D const * (C::* member_fn_ptr)() const;

member_fn_ptr f() {
  return &C::get;
}
