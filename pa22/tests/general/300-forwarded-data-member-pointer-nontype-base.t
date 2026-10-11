// A data member pointer non-type argument can be forwarded from one class
// template to a base class template and still be used in the base member body.

struct record {
  int id;
};

template<class Class, class Type, Type Class::*Ptr>
struct member_base {
  Type &operator()(Class &x) const
  {
    return x.*Ptr;
  }
};

template<class Class, class Type, Type Class::*Ptr>
struct member : member_base<Class, Type, Ptr> {};

int main()
{
  record value = { 7 };
  return member<record, int, &record::id>()(value) == 7 ? 0 : 1;
}
