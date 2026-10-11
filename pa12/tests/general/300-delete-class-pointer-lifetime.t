int c;

struct A {
  A() noexcept
  {
    ++c;
  }

  ~A()
  {
    --c;
  }
};

int main()
{
  A * a = new A();
  delete a;
  return c;
}
