struct Base
{
  int member;
};

struct Padding { int prefix; };

struct Derived : Padding, Base
{
  int extra;
};

int read(Base* object, int Base::*member) { return object->*member; }

int main()
{
  int Base::* base_ptr = &Base::member;
  int Derived::* derived_ptr = base_ptr;
  Derived d;
  d.member = 5;
  d.extra = 7;
  int Base::* inverse = static_cast<int Base::*>(&Derived::extra);
  int Derived::* original = &Derived::extra;
  int Base::* converted = static_cast<int Base::*>(original);
  return d.*derived_ptr == 5 && read(&d, inverse) == 7 &&
         read(&d, converted) == 7 ? 0 : 1;
}
