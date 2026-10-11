struct vector_like
{
  typedef unsigned long storage_type;
};

template<class Container, bool IsConst,
         typename Container::storage_type = 0>
struct iterator
{
};

template<class Container>
int pick(iterator<Container, false>)
{
  return 1;
}

template<class T>
int pick(T)
{
  return 2;
}

int main()
{
  iterator<vector_like, false> value;
  return pick(value) - 1;
}
