struct stream
{
  static int xalloc()
  {
    return 7;
  }
};

template<class Tag>
int get_xalloc_index(int xalloc())
{
  return xalloc();
}

int main()
{
  stream s;
  return get_xalloc_index<int>(s.xalloc) == 7 ? 0 : 1;
}
