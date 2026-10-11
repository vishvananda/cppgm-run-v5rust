template<typename Tag>
struct Box
{
  template<typename T>
  T cast(T value)
  {
    return value;
  }
};

template<typename Tag>
int run(Box<Tag> & box)
{
  return box.cast<int>(4);
}

int main()
{
  Box<int> b;
  return run(b);
}
