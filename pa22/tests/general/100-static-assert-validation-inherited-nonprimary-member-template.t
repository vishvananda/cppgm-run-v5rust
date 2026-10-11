// VALIDATION: compile-pass
// N3485 focus: 7 [dcl.dcl], 10.1 [class.mi], 14.5.2 [temp.mem]

struct first_base
{
  int padding;
};

struct second_base
{
  int value;

  template<class T>
  int read(T) const
  {
    return value;
  }
};

struct derived : first_base, second_base
{
  int test() const
  {
    static_assert(sizeof(int) >= 2, "");
    return read(0);
  }
};

int main()
{
  derived object;
  object.padding = 3;
  object.value = 7;
  return object.test() == 7 ? 0 : 1;
}
