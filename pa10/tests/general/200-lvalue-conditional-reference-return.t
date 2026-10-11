int& choose(bool condition, int& left, int& right)
{
  return condition ? left : right;
}

int main()
{
  int left = 1;
  int right = 2;
  choose(false, left, right) = 3;
  return right - 3;
}
