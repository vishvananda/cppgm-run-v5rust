// VALIDATION: compile-pass
// N3485 focus: 14.6.2.1 [temp.dep.type], 7 [dcl.dcl] static_assert

template<class T>
struct is_lvalue_reference
{
  static const bool value = false;
};

template<class T>
void f()
{
  static_assert(!is_lvalue_reference<T>::value, "");
}

int main()
{
  return 0;
}
