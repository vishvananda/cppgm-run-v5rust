namespace member_template_pointer_target {
struct C {
  int number;
  template<class T> void set(T) {}
  template<class T> operator T() { return number; }
};

template<> C::operator int() { return number + 1; }

template<int (C::*P)()>
int convert(C& value) { return (value.*P)(); }
}

int main()
{
  typedef void (member_template_pointer_target::C::*setter_type)(int);
  setter_type setter = &member_template_pointer_target::C::set;
  (void)setter;
  member_template_pointer_target::C value = {7};
  typedef int (member_template_pointer_target::C::*converter_type)();
  converter_type converter = &member_template_pointer_target::C::operator int;
  return (value.*converter)() != 8 ||
      member_template_pointer_target::convert<
          &member_template_pointer_target::C::operator int>(value) != 8;
}
