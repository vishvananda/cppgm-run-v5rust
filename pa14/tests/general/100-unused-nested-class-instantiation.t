template<class T>
struct Holder {
  struct Unused {
    T value;
  };

  int * first;
  int * last;
};

struct Node {
  Holder<Node> children;
};

int main()
{
  Node node;
  (void)node;
  return 0;
}
