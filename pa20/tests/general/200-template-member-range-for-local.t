template<class T>
struct Box {
  int f()
  {
    int xs[2] = {1, 2};
    int total = 0;
    for (int x : xs)
      total = total + x;
    return total;
  }
};

int main()
{
  Box<int> b;
  return b.f();
}
