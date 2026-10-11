struct X {
  int value;
};

int main()
{
  int X::* p = &X::value;
  (void)p;
  return 0;
}
