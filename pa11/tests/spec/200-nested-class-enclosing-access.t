// N3485 focus: 11.8 [class.access.nest] nested class access to enclosing members
struct Base
{
protected:
  int protected_value;
};

struct Outer : Base
{
private:
  int private_value;

  struct Nested
  {
    static int update(Outer &object)
    {
      object.private_value = 3;
      object.protected_value = 4;
      return object.private_value + object.protected_value;
    }
  };

public:
  Outer() : private_value(0)
  {
    protected_value = 0;
  }

  int run()
  {
    return Nested::update(*this);
  }
};

int main()
{
  Outer object;
  return object.run() == 7 ? 0 : 1;
}
