namespace const_member_function_pointer_field_initializer
{
  struct tester
  {
    void foo() const
    {
    }
  };

  struct holder
  {
    void (tester::*ptr)() const;

    explicit holder(void (tester::*p)() const)
        :
        ptr(p)
    {
    }
  };
}

int main()
{
  const_member_function_pointer_field_initializer::holder h(
      &const_member_function_pointer_field_initializer::tester::foo);
  return 0;
}
