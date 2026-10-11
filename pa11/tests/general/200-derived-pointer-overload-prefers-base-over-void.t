struct Base
{
};

struct Derived : public Base
{
};

int choose(Base const *)
{
  return 1;
}

int choose(void const *)
{
  return 2;
}

int main()
{
  Derived d;
  return choose(&d) == 1 ? 0 : 1;
}
