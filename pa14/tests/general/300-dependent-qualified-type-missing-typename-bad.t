template<class T>
int f(T const volatile *, T::type * = 0)
{
  return 0;
}

struct X
{
  typedef int type;
};

int main()
{
  return f((X *)0);
}
