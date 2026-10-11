int callee(int x)
{
  return x + 7;
}

typedef int (*func_ptr)(int);

func_ptr fp = callee;

int invoke(func_ptr f, int x)
{
  return f(x);
}

int main()
{
  return invoke(fp, 1) == 8 ? 0 : 1;
}
