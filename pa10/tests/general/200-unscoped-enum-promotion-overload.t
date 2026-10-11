enum E
{
  A = 1
};

int which(int)
{
  return 1;
}

int which(unsigned)
{
  return 2;
}

int main()
{
  return which(A) == 1 ? 0 : 1;
}
