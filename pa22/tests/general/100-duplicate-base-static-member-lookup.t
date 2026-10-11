// VALIDATION: compile-pass
// A static member or nested declaration denotes one entity even when its
// declaring base class is reached through multiple nonvirtual subobjects.

struct Base
{
  static int value;

  static int read()
  {
    return value;
  }

  template<typename T>
  static int width()
  {
    return sizeof(T);
  }

  template<typename T>
  struct Nested
  {
    static int width()
    {
      return sizeof(T);
    }
  };

  template<typename T>
  using Alias = T;
};

struct Left : Base
{
};

struct Right : Base
{
};

struct Derived : Left, Right
{
};

int Base::value = 21;

int main()
{
  Derived::Alias<int> zero = 0;
  return Derived::read() + Derived::value + Derived::width<char>() +
             Derived::Nested<char>::width() + zero == 44 ? 0 : 1;
}
