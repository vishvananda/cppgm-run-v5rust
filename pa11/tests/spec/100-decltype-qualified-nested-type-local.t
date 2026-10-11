// N3485 focus: 7.1.6.2 [dcl.type.simple] decltype-specifier

struct holder
{
  typedef int type;
};

int value()
{
  holder source;
  decltype(source)::type result = 0;
  return static_cast<decltype(source)::type>(result);
}

int main()
{
  return value();
}
