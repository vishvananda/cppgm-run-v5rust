template<class T>
struct Box
{
  static unsigned char value;

  static void reset(unsigned char v)
  {
    value = v;
  }

  static unsigned char read()
  {
    return value;
  }
};

template<class T>
unsigned char Box<T>::value = 0u;

int main()
{
  Box<int>::reset(3);
  return Box<int>::read() - 3;
}
