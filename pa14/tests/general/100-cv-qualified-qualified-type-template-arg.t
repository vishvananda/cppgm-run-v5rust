// N3485 focus: 14.3.1 type template arguments.
// A cv-qualified namespace-qualified type must not be treated as a qualified
// template-id merely because the parser carried placeholder qualifier syntax.
namespace ns {
struct S {};

template<class T>
struct box {
  typedef T type;
};
}

int main()
{
  ns::box<const ns::S> value;
  (void)value;
  return 0;
}
