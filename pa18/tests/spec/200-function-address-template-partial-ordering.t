// N3485 focus: 14.5.6.2 [temp.func.order], 14.8.2.2 [temp.deduct.funcaddr]
// When a target function type identifies specializations of two function
// templates, partial ordering selects the more specialized pointer pattern.

template<class T>
int inspect(T)
{
  return 1;
}

template<class T>
int inspect(T*)
{
  return 0;
}

typedef int (*inspection)(int*);

inspection selected = inspect;

int main()
{
  int value = 0;
  return selected(&value);
}
