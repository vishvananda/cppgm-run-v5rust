template<class T> struct Wrap {};

template<class Key, class Alloc = Wrap<const Key> >
struct Box {};

struct Node {};

void accept(Box<const Node *, Wrap<const Node * const> >) {}

int main()
{
  Box<const Node *> box;
  accept(box);
  return 0;
}
