template<class T>
struct Holder {
  struct Temporary {
    union Storage {
      unsigned char byte;
      T value;
      Storage() : byte() {}
      ~Storage() {}
    };

    Storage storage;
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
