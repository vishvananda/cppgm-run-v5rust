struct Base {
  virtual int value() const
  {
    return 1;
  }
};

struct Derived : Base {
  int marker;

  Derived() : marker(7)
  {
  }

  int value() const
  {
    return marker;
  }
};

int read(const Base & b)
{
  return b.value();
}

int main()
{
  return read(Derived()) == 7 ? 0 : 1;
}
