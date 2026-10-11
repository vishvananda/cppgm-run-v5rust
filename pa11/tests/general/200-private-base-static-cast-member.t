struct base {};

class derived : private base
{
public:
  static base const * convert(derived const * value)
  {
    return static_cast<base const *>(value);
  }
};

base const * run(derived const * value)
{
  return derived::convert(value);
}
