namespace std
{
  enum class launch
  {
    async = 1,
    deferred = 2,
    any = async | deferred
  };
}

int main()
{
  return 0;
}
