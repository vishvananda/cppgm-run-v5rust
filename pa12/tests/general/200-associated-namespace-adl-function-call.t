namespace n {
struct token {
  int value;

  token(int x) : value(x) {}
};

int id(token x)
{
  return x.value;
}
}

int main()
{
  return id(n::token(0));
}
