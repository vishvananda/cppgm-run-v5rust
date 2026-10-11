int which(int)
{
  return 1;
}

int which(int *)
{
  return 2;
}

int main()
{
  return which(nullptr) == 2 ? 0 : 1;
}
