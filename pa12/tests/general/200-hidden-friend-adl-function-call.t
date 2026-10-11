struct token {
  int value;

  token(int x) : value(x) {}

  friend int id(token x)
  {
    return x.value;
  }
};

int main()
{
  return id(token(0));
}
