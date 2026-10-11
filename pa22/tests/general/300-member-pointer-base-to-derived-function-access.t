struct Base
{
	int member;
  int f()
  {
    return member;
  }
};

struct Padding { int prefix; };

struct Derived : Padding, Base
{
  int read() { return prefix; }
  int read(int value) { return prefix + value; }
};

int invoke(Base* object, int (Base::*member)())
{
  return (object->*member)();
}

int main()
{
  int (Base::* base_ptr)() = &Base::f;
  int (Derived::* derived_ptr)() = base_ptr;
  Derived d;
  d.member = 5;
  d.prefix = 7;
  int (Base::* inverse)() = static_cast<int (Base::*)()>(&Derived::read);
  int (Derived::* original)() = &Derived::read;
  int (Base::* converted)() = static_cast<int (Base::*)()>(original);
  return (d.*derived_ptr)() == 5 && invoke(&d, base_ptr) == 5 &&
         invoke(&d, inverse) == 7 &&
         invoke(&d, converted) == 7 ? 0 : 1;
}
