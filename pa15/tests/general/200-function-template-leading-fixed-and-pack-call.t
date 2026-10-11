template<class T, class... Args>
int f(T, Args...)
{
  return 9;
}

int main()
{
  return f(1, 2, 3);
}
