template<typename T>
struct InsertBase
{
  using size_type = unsigned long;

  void insert(int) {}
  void insert(int, int) {}

  template<typename It>
  void insert(It, It) {}
};

template<typename T, bool Flag>
struct Insert;

template<typename T>
struct Insert<T, false> : InsertBase<T>
{
  using base_type = InsertBase<T>;
  using typename base_type::size_type;
  using base_type::insert;

  void insert(long) {}
  void insert(int, long) {}
};

template<typename T>
struct Table : Insert<T, false>
{
  using base_type = Insert<T, false>;
  using size_type = typename base_type::size_type;

  template<typename It>
  Table(It first, It last, size_type hint);
};

template<typename T>
template<typename It>
Table<T>::Table(It first, It last, size_type hint)
{
  (void)hint;
  this->template insert<It>(first, last);
}

int main()
{
  int * p = 0;
  Table<int> t(p, p, 0);
  return 0;
}
