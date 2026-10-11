namespace overloaded_member_pointer_target_init {

struct C {
  int value;
  C() : value(0) {}
  void set(int v);
  void set(long v);
};

void C::set(int v)
{
  value = v;
}

void C::set(long v)
{
  value = static_cast<int>(v + 1);
}

}

int main()
{
  typedef void (overloaded_member_pointer_target_init::C::*setter_type)(int);
  setter_type setter = &overloaded_member_pointer_target_init::C::set;
  overloaded_member_pointer_target_init::C c;
  (c.*setter)(4);
  return c.value == 4 ? 0 : 1;
}
