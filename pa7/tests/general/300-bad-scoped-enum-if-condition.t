enum class State { ready };

int inspect(State state)
{
  if (state)
    return 1;
  return 0;
}
