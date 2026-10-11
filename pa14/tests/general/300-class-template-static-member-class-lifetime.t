class holder
{
public:
  holder()
    : value_(7)
  {
  }

  int value() const
  {
    return value_;
  }

private:
  int value_;
};

template<class T>
class stack
{
public:
  static int value()
  {
    return top_.value();
  }

private:
  static holder top_;
};

template<class T>
holder stack<T>::top_;

struct object
{
};

int main()
{
  return stack<object>::value() - 7;
}
