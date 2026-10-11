int invoke(int (*function)(int))
{
  return function(1, 2);
}
