template<bool Forward, class T>
struct iterator
{
  friend iterator<!Forward, T>;
};

iterator<true, int> value;

int main()
{
  return 0;
}
