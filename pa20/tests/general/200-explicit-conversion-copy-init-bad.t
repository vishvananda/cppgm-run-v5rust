struct S
{
  explicit operator bool() const
  {
    return true;
  }
};

bool test()
{
  S s;
  bool b = s;
  return b;
}

int main()
{
  return 0;
}
