struct S {
  explicit operator bool() const { return true; }
};

bool f(S s)
{
  return (bool)s;
}

int main()
{
  return f(S()) ? 0 : 1;
}
