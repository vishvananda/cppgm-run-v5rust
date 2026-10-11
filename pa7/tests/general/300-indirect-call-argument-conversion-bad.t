int call(int (*function)(int *))
{
  return function(1);
}
