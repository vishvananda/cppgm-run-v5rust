template<class T>
struct pair_argument
{
  unsigned long count;
  T value;

  pair_argument(unsigned long count_, T value_) noexcept
    : count(count_), value(value_)
  {
  }
};

template<class T>
pair_argument<T> *make_pair_argument(unsigned long count)
{
  using item = pair_argument<T>;
  return new item(count, T());
}

int main()
{
  pair_argument<int> *result = make_pair_argument<int>(2);
  return result->count == 2 && result->value == 0 ? 0 : 1;
}
