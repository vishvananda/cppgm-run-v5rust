int invoke(int (*function)(int))
{
  return function();
}
