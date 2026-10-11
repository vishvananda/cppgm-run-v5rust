namespace std {
template<class T>
class initializer_list
{
  const T * first;
  unsigned long count;

public:
  unsigned long size() const { return count; }
  const T * begin() const { return first; }
  const T * end() const { return first + count; }
};
}

struct item
{
  int value;
};

struct payload
{
  payload(item, int v) : value(v) {}
  payload(std::initializer_list<item> list, int) : value(list.size()) {}

  int value;
};

struct any_like
{
  template<class T>
  struct holder
  {
    template<class... Args>
    holder(Args&&... args) : value(args...) {}

    template<class U, class... Args>
    holder(std::initializer_list<U> il, Args&&... args) : value(il, args...) {}

    T value;
  };

  template<class T, class... Args>
  T& emplace(Args&&... args)
  {
    holder<T> * raw = new holder<T>(args...);
    content = raw;
    return raw->value;
  }

  template<class T, class U, class... Args>
  T& emplace(std::initializer_list<U> il, Args&&... args)
  {
    holder<T> * raw = new holder<T>(il, args...);
    content = raw;
    return raw->value;
  }

  void * content;
};

int main()
{
  any_like a;
  payload & ref = a.emplace<payload>({item{1}, item{2}}, 7);
  return ref.value == 2 ? 0 : 1;
}
