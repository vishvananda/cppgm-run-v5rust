struct Box {
  template <class T>
  static int twice(T x)
  {
    return x + x;
  }

  int run(int v)
  {
    return [](int x) -> int
           {
             return twice<int>(x);
           }(v);
  }
};

int main()
{
  Box box;
  return box.run(3) - 6;
}
