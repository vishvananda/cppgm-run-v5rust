namespace std
{
typedef unsigned long size_t;

template<class Element>
class initializer_list
{
  const Element * begin_;
  size_t size_;

public:
  const Element * begin() const;
  const Element * end() const;
};
}

struct value
{
  int assign(const char *) { return 1; }
  int assign(std::initializer_list<char>) { return 2; }
};

int main()
{
  value item;
  return item.assign({'1', '2', '3'}) == 2 ? 0 : 1;
}
