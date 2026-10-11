namespace outer
{
  struct holder
  {
    struct id {};
  };

  namespace inner
  {
    template<class T>
    struct box
    {
      static holder::id value;
    };

    template<class T>
    holder::id box<T>::value;
  }
}

int main()
{
  return 0;
}
