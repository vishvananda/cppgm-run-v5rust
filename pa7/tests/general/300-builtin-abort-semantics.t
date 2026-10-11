void stop(bool condition)
{
  if(condition)
    __builtin_abort();
}
