struct Node
{
  int value;
};

template<class T>
struct Base {
  template<class U>
  __attribute__((noinline)) Node* pick(Node* first, Node* second, U selector)
  {
    return selector ? first : second;
  }
};
