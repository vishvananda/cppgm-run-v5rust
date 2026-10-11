template<typename T>
struct B
{
};

template<typename T>
struct D : B<T>
{
};

template<typename T>
int probe(B<T> &)
{
  return 1;
}

int main()
{
  D<int> value;
  return probe(value) == 1 ? 0 : 1;
}
